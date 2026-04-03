use std::env;
use std::process::Command;

struct Args {
    baseline: String,
    candidate: String,
    cutechess: String,
    elo0: f64,
    elo1: f64,
    alpha: f64,
    beta: f64,
    batch_size: u32,
    max_games: u32,
    tc: String,
    openings: Option<String>,
    concurrency: u32,
}

fn main() {
    let args = parse_args();

    let (lower, upper) = sprt_bounds(args.alpha, args.beta);

    let mut total_wins: u32 = 0;
    let mut total_draws: u32 = 0;
    let mut total_losses: u32 = 0;

    while total_wins + total_draws + total_losses < args.max_games {
        let total_games = total_wins + total_draws + total_losses;
        let remaining = args.max_games - total_games;
        let batch = if remaining < args.batch_size {
            remaining
        } else {
            args.batch_size
        };

        let (w, l, d) = run_batch(
            &args.cutechess,
            &args.baseline,
            &args.candidate,
            batch,
            &args.tc,
            args.openings.as_deref(),
            args.concurrency,
        );

        total_wins += w;
        total_losses += l;
        total_draws += d;

        let total = total_wins + total_draws + total_losses;
        let llr = log_likelihood_ratio(total_wins, total_draws, total_losses, args.elo0, args.elo1);
        let score_pct = if total > 0 {
            (total_wins as f64 + total_draws as f64 * 0.5) / total as f64 * 100.0
        } else {
            0.0
        };

        if llr >= upper {
            println!(
                "Games: {}/{} | +{} ={} -{} | Score: {:.1}% | LLR: {:.2} [{:.3}, {:.3}] | Status: accept",
                total, args.max_games, total_wins, total_draws, total_losses, score_pct, llr, lower, upper
            );
            println!(
                "Result: ACCEPT | LLR: {:.2} (>= {:.3}) | Games: {} | +{} ={} -{} | Score: {:.1}%",
                llr, upper, total, total_wins, total_draws, total_losses, score_pct
            );
            std::process::exit(0);
        }

        if llr <= lower {
            println!(
                "Games: {}/{} | +{} ={} -{} | Score: {:.1}% | LLR: {:.2} [{:.3}, {:.3}] | Status: reject",
                total, args.max_games, total_wins, total_draws, total_losses, score_pct, llr, lower, upper
            );
            println!(
                "Result: REJECT | LLR: {:.2} (<= {:.3}) | Games: {} | +{} ={} -{} | Score: {:.1}%",
                llr, lower, total, total_wins, total_draws, total_losses, score_pct
            );
            std::process::exit(1);
        }

        println!(
            "Games: {}/{} | +{} ={} -{} | Score: {:.1}% | LLR: {:.2} [{:.3}, {:.3}] | Status: continue",
            total, args.max_games, total_wins, total_draws, total_losses, score_pct, llr, lower, upper
        );
    }

    let total = total_wins + total_draws + total_losses;
    let llr = log_likelihood_ratio(total_wins, total_draws, total_losses, args.elo0, args.elo1);
    let score_pct = if total > 0 {
        (total_wins as f64 + total_draws as f64 * 0.5) / total as f64 * 100.0
    } else {
        0.0
    };
    println!(
        "Result: INCONCLUSIVE | LLR: {:.2} | Games: {} | +{} ={} -{} | Score: {:.1}%",
        llr, total, total_wins, total_draws, total_losses, score_pct
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let mut baseline = None;
    let mut candidate = None;
    let mut cutechess = None;
    let mut elo0 = 0.0_f64;
    let mut elo1 = 5.0_f64;
    let mut alpha = 0.05_f64;
    let mut beta = 0.05_f64;
    let mut batch_size = 100_u32;
    let mut max_games = 10000_u32;
    let mut tc = String::from("10+0.1");
    let mut openings = None;
    let mut concurrency = 1_u32;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--baseline" => {
                i += 1;
                baseline = Some(args[i].clone());
            }
            "--candidate" => {
                i += 1;
                candidate = Some(args[i].clone());
            }
            "--cutechess" => {
                i += 1;
                cutechess = Some(args[i].clone());
            }
            "--elo0" => {
                i += 1;
                elo0 = args[i].parse::<f64>().expect("invalid elo0");
            }
            "--elo1" => {
                i += 1;
                elo1 = args[i].parse::<f64>().expect("invalid elo1");
            }
            "--alpha" => {
                i += 1;
                alpha = args[i].parse::<f64>().expect("invalid alpha");
            }
            "--beta" => {
                i += 1;
                beta = args[i].parse::<f64>().expect("invalid beta");
            }
            "--batch-size" => {
                i += 1;
                batch_size = args[i].parse::<u32>().expect("invalid batch-size");
            }
            "--max-games" => {
                i += 1;
                max_games = args[i].parse::<u32>().expect("invalid max-games");
            }
            "--tc" => {
                i += 1;
                tc = args[i].clone();
            }
            "--openings" => {
                i += 1;
                openings = Some(args[i].clone());
            }
            "--concurrency" => {
                i += 1;
                concurrency = args[i].parse::<u32>().expect("invalid concurrency");
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let baseline = baseline.unwrap_or_else(|| {
        eprintln!("Error: --baseline is required");
        print_usage();
        std::process::exit(1);
    });
    let candidate = candidate.unwrap_or_else(|| {
        eprintln!("Error: --candidate is required");
        print_usage();
        std::process::exit(1);
    });
    let cutechess = cutechess.unwrap_or_else(|| {
        eprintln!("Error: --cutechess is required");
        print_usage();
        std::process::exit(1);
    });

    Args {
        baseline,
        candidate,
        cutechess,
        elo0,
        elo1,
        alpha,
        beta,
        batch_size,
        max_games,
        tc,
        openings,
        concurrency,
    }
}

fn print_usage() {
    eprintln!(
        "Usage: sprt --baseline <path> --candidate <path> --cutechess <path> \
         [--elo0 <f64>] [--elo1 <f64>] [--alpha <f64>] [--beta <f64>] \
         [--tc <string>] [--batch-size <N>] [--max-games <N>] \
         [--openings <path>] [--concurrency <N>]"
    );
    eprintln!();
    eprintln!("Required:");
    eprintln!("  --baseline <path>     Path to baseline engine binary");
    eprintln!("  --candidate <path>    Path to candidate engine binary");
    eprintln!("  --cutechess <path>    Path to CuteChess-CLI binary");
    eprintln!();
    eprintln!("Optional:");
    eprintln!("  --elo0 <f64>          Null hypothesis Elo bound (default: 0.0)");
    eprintln!("  --elo1 <f64>          Alternative hypothesis Elo bound (default: 5.0)");
    eprintln!("  --alpha <f64>         Type I error rate (default: 0.05)");
    eprintln!("  --beta <f64>          Type II error rate (default: 0.05)");
    eprintln!("  --tc <string>         Time control string (default: 10+0.1)");
    eprintln!("  --batch-size <N>      Games per batch (default: 100)");
    eprintln!("  --max-games <N>       Maximum total games (default: 10000)");
    eprintln!("  --openings <path>     Openings file path");
    eprintln!("  --concurrency <N>     Concurrent games (default: 1)");
}

fn run_batch(
    cutechess: &str,
    baseline: &str,
    candidate: &str,
    games: u32,
    tc: &str,
    openings: Option<&str>,
    concurrency: u32,
) -> (u32, u32, u32) {
    let rounds = games.div_ceil(2);

    let mut cmd = Command::new(cutechess);
    cmd.arg("-engine")
        .arg(format!("name=Candidate cmd={candidate}"))
        .arg("-engine")
        .arg(format!("name=Baseline cmd={baseline}"))
        .arg("-each")
        .arg(format!("proto=uci tc={tc}"))
        .arg("-games")
        .arg(games.to_string())
        .arg("-rounds")
        .arg(rounds.to_string())
        .arg("-repeat")
        .arg("-concurrency")
        .arg(concurrency.to_string())
        .arg("-ratinginterval")
        .arg("0");

    if let Some(path) = openings {
        cmd.arg("-openingfile").arg(path);
    }

    let output = cmd.output().unwrap_or_else(|e| {
        eprintln!("Failed to run CuteChess-CLI: {e}");
        std::process::exit(1);
    });

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines().rev() {
        if let Some(result) = parse_score_line(line) {
            return result;
        }
    }

    eprintln!("No score line found in CuteChess-CLI output");
    eprintln!("stdout: {stdout}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("stderr: {stderr}");
    std::process::exit(1);
}

fn parse_score_line(line: &str) -> Option<(u32, u32, u32)> {
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

fn elo_to_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf(-elo / 400.0))
}

fn log_likelihood_ratio(wins: u32, draws: u32, losses: u32, elo0: f64, elo1: f64) -> f64 {
    let n = (wins + draws + losses) as f64;
    if n == 0.0 {
        return 0.0;
    }

    let w = wins as f64;
    let d = draws as f64;
    let l = losses as f64;

    let observed_score = (w + d * 0.5) / n;

    let variance = (w * (1.0 - observed_score).powi(2)
        + d * (0.5 - observed_score).powi(2)
        + l * observed_score.powi(2))
        / n;

    if variance < 1e-12 {
        return 0.0;
    }

    let s0 = elo_to_score(elo0);
    let s1 = elo_to_score(elo1);

    0.5 * n * (s1 - s0) * (2.0 * observed_score - s0 - s1) / variance
}

fn sprt_bounds(alpha: f64, beta: f64) -> (f64, f64) {
    let lower = (beta / (1.0 - alpha)).ln();
    let upper = ((1.0 - beta) / alpha).ln();
    (lower, upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprt_bounds() {
        let (lower, upper) = sprt_bounds(0.05, 0.05);
        assert!((lower - (-2.944)).abs() < 0.01, "lower={lower}");
        assert!((upper - 2.944).abs() < 0.01, "upper={upper}");
    }

    #[test]
    fn test_elo_to_score_zero() {
        let s = elo_to_score(0.0);
        assert!((s - 0.5).abs() < 1e-10, "s={s}");
    }

    #[test]
    fn test_elo_to_score_positive() {
        let s = elo_to_score(5.0);
        assert!(s > 0.5, "s={s}");
        assert!((s - 0.5072).abs() < 0.001, "s={s}");
    }

    #[test]
    fn test_llr_equal_score() {
        let llr = log_likelihood_ratio(100, 200, 100, 0.0, 5.0);
        assert!(llr.abs() < 0.5, "llr={llr}");
    }

    #[test]
    fn test_llr_positive() {
        let llr = log_likelihood_ratio(150, 200, 100, 0.0, 5.0);
        assert!(llr > 0.0, "llr={llr}");
    }

    #[test]
    fn test_llr_negative() {
        let llr = log_likelihood_ratio(100, 200, 150, 0.0, 5.0);
        assert!(llr < 0.0, "llr={llr}");
    }

    #[test]
    fn test_llr_zero_games() {
        let llr = log_likelihood_ratio(0, 0, 0, 0.0, 5.0);
        assert!((llr - 0.0).abs() < 1e-10, "llr={llr}");
    }

    #[test]
    fn test_parse_score_line_valid() {
        let line = "Score of Candidate vs Baseline: 120 - 100 - 280 [0.520]";
        let result = parse_score_line(line);
        assert_eq!(result, Some((120, 100, 280)));
    }

    #[test]
    fn test_parse_score_line_invalid() {
        let result = parse_score_line("not a score line");
        assert_eq!(result, None);
    }
}
