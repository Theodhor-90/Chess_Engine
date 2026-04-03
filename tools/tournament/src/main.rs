use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{self, Command, Stdio};

struct Args {
    engine: String,
    cutechess: String,
    config: String,
    games: Option<u32>,
    concurrency: Option<u32>,
    tc: Option<String>,
    openings: Option<String>,
}

fn print_usage() {
    eprintln!("Usage: tournament --engine <path> --cutechess <path> --config <path> [options]");
    eprintln!();
    eprintln!("Required:");
    eprintln!("  --engine <path>       Path to our engine binary");
    eprintln!("  --cutechess <path>    Path to CuteChess-CLI binary");
    eprintln!("  --config <path>       Path to engines.toml config");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --games <N>           Games per engine pair (default from config)");
    eprintln!("  --concurrency <N>     Concurrent games (default from config)");
    eprintln!("  --tc <string>         Time control override (e.g., 10+0.1)");
    eprintln!("  --openings <path>     Opening book/FEN file override");
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut engine = None;
    let mut cutechess = None;
    let mut config = None;
    let mut games = None;
    let mut concurrency = None;
    let mut tc = None;
    let mut openings = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--engine" => {
                i += 1;
                engine = Some(args[i].clone());
            }
            "--cutechess" => {
                i += 1;
                cutechess = Some(args[i].clone());
            }
            "--config" => {
                i += 1;
                config = Some(args[i].clone());
            }
            "--games" => {
                i += 1;
                games = Some(args[i].parse::<u32>().unwrap());
            }
            "--concurrency" => {
                i += 1;
                concurrency = Some(args[i].parse::<u32>().unwrap());
            }
            "--tc" => {
                i += 1;
                tc = Some(args[i].clone());
            }
            "--openings" => {
                i += 1;
                openings = Some(args[i].clone());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    let engine = match engine {
        Some(e) => e,
        None => {
            eprintln!("Error: --engine is required");
            print_usage();
            process::exit(1);
        }
    };
    let cutechess = match cutechess {
        Some(c) => c,
        None => {
            eprintln!("Error: --cutechess is required");
            print_usage();
            process::exit(1);
        }
    };
    let config = match config {
        Some(c) => c,
        None => {
            eprintln!("Error: --config is required");
            print_usage();
            process::exit(1);
        }
    };

    Args {
        engine,
        cutechess,
        config,
        games,
        concurrency,
        tc,
        openings,
    }
}

struct Defaults {
    games: u32,
    concurrency: u32,
    openings_file: String,
    openings_format: String,
    repeat: bool,
}

struct ReferenceEngine {
    name: String,
    cmd: String,
    tc: String,
    options: HashMap<String, String>,
}

struct Config {
    defaults: Defaults,
    engines: Vec<ReferenceEngine>,
}

fn parse_config(path: &str) -> Config {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading config file '{}': {}", path, e);
        process::exit(1);
    });

    let mut defaults = Defaults {
        games: 500,
        concurrency: 1,
        openings_file: String::new(),
        openings_format: "epd".to_string(),
        repeat: true,
    };

    let mut engines: Vec<ReferenceEngine> = Vec::new();

    enum Section {
        None,
        Defaults,
        Engine,
        EngineOptions,
    }

    let mut section = Section::None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "[defaults]" {
            section = Section::Defaults;
            continue;
        }
        if line == "[[engine]]" {
            engines.push(ReferenceEngine {
                name: String::new(),
                cmd: String::new(),
                tc: String::new(),
                options: HashMap::new(),
            });
            section = Section::Engine;
            continue;
        }
        if line == "[engine.options]" {
            section = Section::EngineOptions;
            continue;
        }

        if let Some((key, val)) = parse_kv(line) {
            match section {
                Section::None => {}
                Section::Defaults => match key.as_str() {
                    "games" => defaults.games = val.parse().unwrap(),
                    "concurrency" => defaults.concurrency = val.parse().unwrap(),
                    "openings_file" => defaults.openings_file = val,
                    "openings_format" => defaults.openings_format = val,
                    "repeat" => defaults.repeat = val == "true",
                    _ => {}
                },
                Section::Engine => {
                    if let Some(eng) = engines.last_mut() {
                        match key.as_str() {
                            "name" => eng.name = val,
                            "cmd" => eng.cmd = val,
                            "tc" => eng.tc = val,
                            _ => {}
                        }
                    }
                }
                Section::EngineOptions => {
                    if let Some(eng) = engines.last_mut() {
                        eng.options.insert(key, val);
                    }
                }
            }
        }
    }

    Config { defaults, engines }
}

fn parse_kv(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let key = parts[0].trim().trim_matches('"').to_string();
    let val = parts[1].trim().trim_matches('"').to_string();
    Some((key, val))
}

struct TournamentParams<'a> {
    cutechess: &'a str,
    engine_path: &'a str,
    games: u32,
    concurrency: u32,
    tc_override: Option<&'a str>,
    openings_file: &'a str,
    openings_format: &'a str,
    repeat: bool,
}

struct TournamentResult {
    opponent: String,
    games: u32,
    wins: u32,
    draws: u32,
    losses: u32,
    elo: f64,
    elo_error: f64,
}

fn parse_score_line(line: &str) -> Option<(u32, u32, u32)> {
    // Format: "Score of X vs Y: W - L - D  [pct]"
    let colon_idx = line.find(':')?;
    let after_colon = line[colon_idx + 1..].trim();
    let bracket_idx = after_colon.find('[')?;
    let scores_part = after_colon[..bracket_idx].trim();

    let parts: Vec<&str> = scores_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let w: u32 = parts[0].trim().parse().ok()?;
    let l: u32 = parts[1].trim().parse().ok()?;
    let d: u32 = parts[2].trim().parse().ok()?;

    Some((w, l, d))
}

fn parse_elo_line(line: &str) -> Option<(f64, f64)> {
    // Format: "Elo difference: X.XX +/- Y.YY"
    let colon_idx = line.find(':')?;
    let after_colon = line[colon_idx + 1..].trim();

    let parts: Vec<&str> = after_colon.split("+/-").collect();
    if parts.len() != 2 {
        return None;
    }

    let elo: f64 = parts[0].trim().parse().ok()?;
    let error: f64 = parts[1].trim().parse().ok()?;

    Some((elo, error))
}

fn compute_elo_from_wdl(w: u32, d: u32, l: u32) -> (f64, f64) {
    let n = (w + d + l) as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }

    let score = (w as f64 + d as f64 * 0.5) / n;

    let elo = if score <= 0.0 || score >= 1.0 {
        if score <= 0.0 {
            -999.0
        } else {
            999.0
        }
    } else {
        -400.0 * (1.0 / score - 1.0).log10()
    };

    let se = (score * (1.0 - score) / n).sqrt();
    let elo_error = if score > 0.001 && score < 0.999 {
        let score_upper = (score + se).min(0.999);
        let score_lower = (score - se).max(0.001);
        let elo_upper = -400.0 * (1.0 / score_upper - 1.0).log10();
        let elo_lower = -400.0 * (1.0 / score_lower - 1.0).log10();
        1.96 * (elo_upper - elo_lower) / 2.0
    } else {
        0.0
    };

    (elo, elo_error)
}

fn run_tournament(params: &TournamentParams, opponent: &ReferenceEngine) -> TournamentResult {
    let tc = params.tc_override.unwrap_or(&opponent.tc);
    let rounds = params.games / 2;

    let mut cmd = Command::new(params.cutechess);
    cmd.arg("-engine")
        .arg(format!("name=OurEngine cmd={}", params.engine_path));

    let mut ref_arg = format!("name={} cmd={}", opponent.name, opponent.cmd);
    for (key, val) in &opponent.options {
        ref_arg.push_str(&format!(" option.{}={}", key, val));
    }
    cmd.arg("-engine").arg(&ref_arg);

    cmd.arg("-each")
        .arg(format!("proto=uci tc={}", tc))
        .arg("-games")
        .arg(params.games.to_string())
        .arg("-rounds")
        .arg(rounds.to_string());

    if params.repeat {
        cmd.arg("-repeat");
    }

    if !params.openings_file.is_empty() {
        cmd.arg("-openingfile").arg(params.openings_file);
        cmd.arg("-openingformat").arg(params.openings_format);
    }

    cmd.arg("-concurrency").arg(params.concurrency.to_string());
    cmd.arg("-ratinginterval").arg("0");

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    eprintln!(
        "Starting tournament: OurEngine vs {} ({} games, tc={})",
        opponent.name, params.games, tc
    );

    let mut child = cmd.spawn().unwrap_or_else(|e| {
        eprintln!("Error spawning CuteChess-CLI '{}': {}", params.cutechess, e);
        process::exit(1);
    });

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut draws = 0u32;
    let mut elo_diff = None;
    let mut elo_err = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.starts_with("Score of") {
            if let Some((w, l, d)) = parse_score_line(&line) {
                wins = w;
                losses = l;
                draws = d;
            }
        }

        if line.starts_with("Elo difference") {
            if let Some((e, err)) = parse_elo_line(&line) {
                elo_diff = Some(e);
                elo_err = Some(err);
            }
        }
    }

    let status = child.wait().unwrap();
    if !status.success() {
        eprintln!(
            "Warning: CuteChess-CLI exited with status {} for {}",
            status, opponent.name
        );
    }

    let (elo, error) = match (elo_diff, elo_err) {
        (Some(e), Some(err)) => (e, err),
        _ => {
            eprintln!(
                "Warning: Could not parse CuteChess-CLI Elo output for {}, computing from W/D/L",
                opponent.name
            );
            compute_elo_from_wdl(wins, draws, losses)
        }
    };

    TournamentResult {
        opponent: opponent.name.clone(),
        games: wins + draws + losses,
        wins,
        draws,
        losses,
        elo,
        elo_error: error,
    }
}

fn print_result(result: &TournamentResult) {
    println!("=== Tournament: OurEngine vs {} ===", result.opponent);
    println!("Games: {}", result.games);
    println!(
        "Score: +{} ={} -{} ({:.1}%)",
        result.wins,
        result.draws,
        result.losses,
        if result.games > 0 {
            (result.wins as f64 + result.draws as f64 * 0.5) / result.games as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Elo difference: {:+.1} +/- {:.1} (95% CI)",
        result.elo, result.elo_error
    );
    println!();
}

fn print_summary(results: &[TournamentResult]) {
    println!("=== Summary ===");
    println!(
        "{:<15} {:>5}   {:>7}   {:>7}   {:>10}",
        "Opponent", "Games", "Score", "Elo", "95% CI"
    );
    for r in results {
        let pct = if r.games > 0 {
            (r.wins as f64 + r.draws as f64 * 0.5) / r.games as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "{:<15} {:>5}   {:>6.1}%   {:>+6.1}   +/- {:.1}",
            r.opponent, r.games, pct, r.elo, r.elo_error
        );
    }
}

fn main() {
    let args = parse_args();
    let config = parse_config(&args.config);

    let games = args.games.unwrap_or(config.defaults.games);
    let concurrency = args.concurrency.unwrap_or(config.defaults.concurrency);
    let openings_file = args
        .openings
        .as_deref()
        .unwrap_or(&config.defaults.openings_file);
    let openings_format = &config.defaults.openings_format;

    let params = TournamentParams {
        cutechess: &args.cutechess,
        engine_path: &args.engine,
        games,
        concurrency,
        tc_override: args.tc.as_deref(),
        openings_file,
        openings_format,
        repeat: config.defaults.repeat,
    };

    let mut results = Vec::new();

    for engine in &config.engines {
        let result = run_tournament(&params, engine);
        print_result(&result);
        results.push(result);
    }

    print_summary(&results);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_score_line_valid() {
        let line = "Score of OurEngine vs Stockfish: 120 - 120 - 260  [0.500]";
        let (w, l, d) = parse_score_line(line).unwrap();
        assert_eq!(w, 120);
        assert_eq!(l, 120);
        assert_eq!(d, 260);
    }

    #[test]
    fn parse_score_line_different_numbers() {
        let line = "Score of EngA vs EngB: 250 - 100 - 150  [0.650]";
        let (w, l, d) = parse_score_line(line).unwrap();
        assert_eq!(w, 250);
        assert_eq!(l, 100);
        assert_eq!(d, 150);
    }

    #[test]
    fn parse_score_line_invalid() {
        let line = "Not a score line";
        assert!(parse_score_line(line).is_none());
    }

    #[test]
    fn parse_elo_line_valid() {
        let line = "Elo difference: 41.80 +/- 29.10";
        let (elo, err) = parse_elo_line(line).unwrap();
        assert!((elo - 41.80).abs() < 0.01);
        assert!((err - 29.10).abs() < 0.01);
    }

    #[test]
    fn parse_elo_line_negative() {
        let line = "Elo difference: -15.30 +/- 25.50";
        let (elo, err) = parse_elo_line(line).unwrap();
        assert!((elo - -15.30).abs() < 0.01);
        assert!((err - 25.50).abs() < 0.01);
    }

    #[test]
    fn parse_elo_line_invalid() {
        let line = "Some other output";
        assert!(parse_elo_line(line).is_none());
    }

    #[test]
    fn compute_elo_from_equal_score() {
        let (elo, _) = compute_elo_from_wdl(100, 300, 100);
        assert!(
            elo.abs() < 1.0,
            "Equal score should give ~0 Elo, got {}",
            elo
        );
    }

    #[test]
    fn compute_elo_positive_score() {
        // 60% score -> positive Elo
        let (elo, error) = compute_elo_from_wdl(200, 200, 100);
        assert!(elo > 0.0, "60% score should give positive Elo, got {}", elo);
        assert!(error > 0.0, "Error margin should be positive");
    }

    #[test]
    fn compute_elo_negative_score() {
        // 40% score -> negative Elo
        let (elo, error) = compute_elo_from_wdl(100, 200, 200);
        assert!(elo < 0.0, "40% score should give negative Elo, got {}", elo);
        assert!(error > 0.0, "Error margin should be positive");
    }

    #[test]
    fn compute_elo_zero_games() {
        let (elo, error) = compute_elo_from_wdl(0, 0, 0);
        assert!((elo - 0.0).abs() < f64::EPSILON);
        assert!((error - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_elo_500_games_ci_reasonable() {
        // With 500 games at ~50%, CI should be roughly ±30
        let (_, error) = compute_elo_from_wdl(125, 250, 125);
        assert!(
            error > 10.0 && error < 60.0,
            "95% CI for 500 games at 50% should be reasonable, got ±{:.1}",
            error
        );
    }

    #[test]
    fn parse_config_file() {
        let config = parse_config("engines.toml");
        assert_eq!(config.defaults.games, 500);
        assert_eq!(config.defaults.concurrency, 1);
        assert_eq!(config.engines.len(), 3);
        assert_eq!(config.engines[0].name, "Stockfish");
        assert_eq!(config.engines[1].name, "Ethereal");
        assert_eq!(config.engines[2].name, "Rustic");
        assert_eq!(
            config.engines[0].options.get("Skill Level"),
            Some(&"5".to_string())
        );
    }
}
