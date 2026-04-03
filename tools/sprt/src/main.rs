use std::env;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use chess_board::Position;
use chess_movegen::generate_legal_moves;
use chess_types::{Color, Piece, PieceKind, Square};

struct Args {
    baseline: String,
    candidate: String,
    openings: String,
    elo0: f64,
    elo1: f64,
    alpha: f64,
    beta: f64,
    movetime: u64,
    maxgames: u32,
    maxmoves: u32,
}

#[derive(Debug)]
enum GameResult {
    Win,
    Draw,
    Loss,
}

struct EngineHandle {
    child: Child,
    writer: BufWriter<std::process::ChildStdin>,
    rx: mpsc::Receiver<String>,
}

fn main() {
    let args = parse_args();
    let openings = load_openings(&args.openings);

    let (lower, upper) = sprt_bounds(args.alpha, args.beta);
    println!(
        "SPRT: elo0={:.1} elo1={:.1} alpha={:.2} beta={:.2}",
        args.elo0, args.elo1, args.alpha, args.beta
    );
    println!("Bounds: lower={lower:.3} upper={upper:.3}");

    let mut wins: u32 = 0;
    let mut draws: u32 = 0;
    let mut losses: u32 = 0;
    let mut game_num: u32 = 0;

    while game_num < args.maxgames {
        let opening_index = (game_num / 2) as usize % openings.len();
        let opening_fen = &openings[opening_index];
        let baseline_is_white = game_num.is_multiple_of(2);

        let result = play_game(
            &args.baseline,
            &args.candidate,
            opening_fen,
            args.movetime,
            args.maxmoves,
            baseline_is_white,
        );

        match result {
            GameResult::Win => wins += 1,
            GameResult::Draw => draws += 1,
            GameResult::Loss => losses += 1,
        }

        game_num += 1;
        let total = wins + draws + losses;
        let score_pct = if total > 0 {
            (wins as f64 + draws as f64 * 0.5) / total as f64 * 100.0
        } else {
            0.0
        };

        let llr = log_likelihood_ratio(wins, draws, losses, args.elo0, args.elo1);
        println!(
            "Game {}/{}: +{} ={} -{} (score={:.1}%) LLR={:.2} [{:.3}, {:.3}]",
            game_num, args.maxgames, wins, draws, losses, score_pct, llr, lower, upper
        );

        if llr >= upper {
            println!("Result: H1 accepted (candidate is stronger)");
            println!(
                "Final: +{wins} ={draws} -{losses} (score={score_pct:.1}%) LLR={llr:.2} after {game_num} games"
            );
            return;
        }
        if llr <= lower {
            println!("Result: H0 accepted (no significant difference)");
            println!(
                "Final: +{wins} ={draws} -{losses} (score={score_pct:.1}%) LLR={llr:.2} after {game_num} games"
            );
            return;
        }
    }

    let total = wins + draws + losses;
    let score_pct = if total > 0 {
        (wins as f64 + draws as f64 * 0.5) / total as f64 * 100.0
    } else {
        0.0
    };
    let llr = log_likelihood_ratio(wins, draws, losses, args.elo0, args.elo1);
    println!("Result: Inconclusive (max games reached)");
    println!(
        "Final: +{wins} ={draws} -{losses} (score={score_pct:.1}%) LLR={llr:.2} after {game_num} games"
    );
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let mut baseline = None;
    let mut candidate = None;
    let mut openings = None;
    let mut elo0 = None;
    let mut elo1 = None;
    let mut alpha = None;
    let mut beta = None;
    let mut movetime = None;
    let mut maxgames = 10000u32;
    let mut maxmoves = 300u32;

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
            "--openings" => {
                i += 1;
                openings = Some(args[i].clone());
            }
            "--elo0" => {
                i += 1;
                elo0 = Some(args[i].parse::<f64>().expect("invalid elo0"));
            }
            "--elo1" => {
                i += 1;
                elo1 = Some(args[i].parse::<f64>().expect("invalid elo1"));
            }
            "--alpha" => {
                i += 1;
                alpha = Some(args[i].parse::<f64>().expect("invalid alpha"));
            }
            "--beta" => {
                i += 1;
                beta = Some(args[i].parse::<f64>().expect("invalid beta"));
            }
            "--movetime" => {
                i += 1;
                movetime = Some(args[i].parse::<u64>().expect("invalid movetime"));
            }
            "--maxgames" => {
                i += 1;
                maxgames = args[i].parse::<u32>().expect("invalid maxgames");
            }
            "--maxmoves" => {
                i += 1;
                maxmoves = args[i].parse::<u32>().expect("invalid maxmoves");
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
        print_usage();
        std::process::exit(1);
    });
    let candidate = candidate.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let openings = openings.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let elo0 = elo0.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let elo1 = elo1.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let alpha = alpha.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let beta = beta.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let movetime = movetime.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });

    Args {
        baseline,
        candidate,
        openings,
        elo0,
        elo1,
        alpha,
        beta,
        movetime,
        maxgames,
        maxmoves,
    }
}

fn print_usage() {
    eprintln!(
        "Usage: sprt --baseline <path> --candidate <path> --openings <path> \
         --elo0 <f64> --elo1 <f64> --alpha <f64> --beta <f64> \
         --movetime <ms> [--maxgames <N>] [--maxmoves <N>]"
    );
}

fn load_openings(path: &str) -> Vec<String> {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Failed to read {path}: {e}");
        std::process::exit(1);
    });
    let openings: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    assert!(!openings.is_empty(), "No openings loaded from {path}");
    openings
}

fn spawn_engine(path: &str) -> EngineHandle {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn engine '{path}': {e}"));

    let stdin = child.stdin.take().expect("failed to open engine stdin");
    let stdout = child.stdout.take().expect("failed to open engine stdout");
    let mut writer = BufWriter::new(stdin);

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(buf.trim().to_string()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    send_line(&mut writer, "uci");
    wait_for_line(&rx, "uciok", Duration::from_secs(5));
    send_line(&mut writer, "isready");
    wait_for_line(&rx, "readyok", Duration::from_secs(5));

    EngineHandle { child, writer, rx }
}

fn send_line(writer: &mut impl Write, line: &str) {
    writeln!(writer, "{line}").expect("failed to write to engine");
    writer.flush().expect("failed to flush engine stdin");
}

fn wait_for_line(rx: &mpsc::Receiver<String>, expected: &str, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            panic!("timeout waiting for '{expected}' from engine");
        }
        match rx.recv_timeout(remaining) {
            Ok(line) if line == expected => return,
            Ok(_) => continue,
            Err(_) => panic!("engine closed stdout while waiting for '{expected}'"),
        }
    }
}

fn wait_for_bestmove(
    rx: &mpsc::Receiver<String>,
    timeout: Duration,
) -> Result<String, &'static str> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("timeout");
        }
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if let Some(rest) = line.strip_prefix("bestmove") {
                    let rest = rest.trim();
                    let mv = rest.split_whitespace().next().unwrap_or("0000");
                    return Ok(mv.to_string());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return Err("timeout"),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("disconnected"),
        }
    }
}

fn validate_move(pos: &mut Position, move_str: &str) -> Option<chess_types::Move> {
    let legal_moves = generate_legal_moves(pos);
    for mv in &legal_moves {
        if format!("{mv}") == move_str {
            return Some(*mv);
        }
    }
    None
}

fn find_king_square(pos: &Position, color: Color) -> Square {
    let king = Piece::new(color, PieceKind::King);
    let bb = pos.piece_bitboard(king);
    let sq_idx = bb
        .lsb()
        .unwrap_or_else(|| panic!("no king found for {color:?}"));
    Square::new(sq_idx as u8).unwrap()
}

fn is_in_check(pos: &Position) -> bool {
    let side = pos.side_to_move();
    let king_sq = find_king_square(pos, side);
    pos.is_square_attacked(king_sq, side.opposite())
}

fn is_insufficient_material(pos: &Position) -> bool {
    let white_pieces = pos.occupied_by(Color::White);
    let black_pieces = pos.occupied_by(Color::Black);
    let total = white_pieces.pop_count() + black_pieces.pop_count();

    if total == 2 {
        return true;
    }

    if total == 3 {
        for color in [Color::White, Color::Black] {
            let knights = pos.piece_bitboard(Piece::new(color, PieceKind::Knight));
            let bishops = pos.piece_bitboard(Piece::new(color, PieceKind::Bishop));
            if knights.pop_count() == 1 || bishops.pop_count() == 1 {
                return true;
            }
        }
    }

    if total == 4 {
        let wb = pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop));
        let bb = pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop));
        if wb.pop_count() == 1 && bb.pop_count() == 1 {
            let w_sq = Square::new(wb.lsb().unwrap() as u8).unwrap();
            let b_sq = Square::new(bb.lsb().unwrap() as u8).unwrap();
            let w_dark = (w_sq.file() as u8 + w_sq.rank() as u8) % 2;
            let b_dark = (b_sq.file() as u8 + b_sq.rank() as u8) % 2;
            if w_dark == b_dark {
                return true;
            }
        }
    }

    false
}

fn count_repetitions(hash_history: &[u64], current_hash: u64) -> u32 {
    let mut count = 0;
    for &h in hash_history {
        if h == current_hash {
            count += 1;
        }
    }
    count
}

#[allow(clippy::too_many_arguments)]
fn play_game(
    baseline_path: &str,
    candidate_path: &str,
    opening_fen: &str,
    movetime: u64,
    max_moves: u32,
    baseline_is_white: bool,
) -> GameResult {
    let mut pos = Position::from_fen(opening_fen).unwrap_or_else(|e| {
        panic!("invalid opening FEN '{opening_fen}': {e:?}");
    });

    let mut hash_history: Vec<u64> = vec![pos.hash()];
    let mut move_history: Vec<String> = Vec::new();
    let mut ply_count = 0u32;

    let (white_path, black_path) = if baseline_is_white {
        (baseline_path, candidate_path)
    } else {
        (candidate_path, baseline_path)
    };

    let mut white_engine = spawn_engine(white_path);
    let mut black_engine = spawn_engine(black_path);

    let timeout = Duration::from_millis(movetime * 10);

    let game_result = loop {
        if ply_count >= max_moves {
            break GameResult::Draw;
        }

        let side = pos.side_to_move();
        let engine = if side == Color::White {
            &mut white_engine
        } else {
            &mut black_engine
        };

        send_line(&mut engine.writer, "isready");
        wait_for_line(&engine.rx, "readyok", Duration::from_secs(5));

        let pos_cmd = if move_history.is_empty() {
            format!("position fen {opening_fen}")
        } else {
            format!(
                "position fen {opening_fen} moves {}",
                move_history.join(" ")
            )
        };
        send_line(&mut engine.writer, &pos_cmd);

        let go_cmd = format!("go movetime {movetime}");
        send_line(&mut engine.writer, &go_cmd);

        let bestmove_str = match wait_for_bestmove(&engine.rx, timeout) {
            Ok(s) => s,
            Err(_) => break GameResult::Draw,
        };

        if bestmove_str == "0000" || bestmove_str == "(none)" {
            break GameResult::Draw;
        }

        let mv = match validate_move(&mut pos, &bestmove_str) {
            Some(mv) => mv,
            None => break GameResult::Draw,
        };

        let _undo = pos.make_move(mv);
        move_history.push(bestmove_str);
        ply_count += 1;
        hash_history.push(pos.hash());

        let legal_moves = generate_legal_moves(&mut pos);
        if legal_moves.is_empty() {
            if is_in_check(&pos) {
                let winner_is_white = pos.side_to_move() == Color::Black;
                let candidate_wins = if baseline_is_white {
                    !winner_is_white
                } else {
                    winner_is_white
                };
                break if candidate_wins {
                    GameResult::Win
                } else {
                    GameResult::Loss
                };
            } else {
                break GameResult::Draw;
            }
        }

        if pos.halfmove_clock() >= 100 {
            break GameResult::Draw;
        }

        if count_repetitions(&hash_history, pos.hash()) >= 3 {
            break GameResult::Draw;
        }

        if is_insufficient_material(&pos) {
            break GameResult::Draw;
        }
    };

    send_line(&mut white_engine.writer, "quit");
    send_line(&mut black_engine.writer, "quit");

    let _ = white_engine.child.wait();
    let _ = black_engine.child.wait();

    game_result
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
