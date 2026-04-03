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
    engine: String,
    openings: String,
    games: u32,
    movetime: u64,
    max_moves: u32,
}

#[derive(Debug)]
enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

#[derive(Debug)]
#[allow(dead_code)]
enum GameEndReason {
    Checkmate,
    Stalemate,
    FiftyMoveRule,
    ThreefoldRepetition,
    InsufficientMaterial,
    MaxMovesReached,
    Crash {
        side: &'static str,
        exit_status: Option<i32>,
    },
    IllegalMove {
        side: &'static str,
        move_str: String,
    },
    TimeLoss {
        side: &'static str,
    },
    EngineResigned {
        side: &'static str,
    },
}

struct GameOutcome {
    game_num: u32,
    opening_fen: String,
    result: Option<GameResult>,
    reason: GameEndReason,
    ply_count: u32,
}

struct Stats {
    total_games: u32,
    white_wins: u32,
    black_wins: u32,
    draws: u32,
    crashes: u32,
    illegal_moves: u32,
    time_losses: u32,
    assertion_failures: u32,
}

fn main() {
    let args = parse_args();
    let openings = load_openings(&args.openings);
    let timeout = Duration::from_millis(args.movetime * 10);

    let mut stats = Stats {
        total_games: 0,
        white_wins: 0,
        black_wins: 0,
        draws: 0,
        crashes: 0,
        illegal_moves: 0,
        time_losses: 0,
        assertion_failures: 0,
    };

    for game_num in 0..args.games {
        let opening_index = (game_num / 2) as usize % openings.len();
        let opening_fen = &openings[opening_index];

        let outcome = play_game(
            &args.engine,
            opening_fen,
            game_num,
            &timeout,
            args.max_moves,
        );
        print_game_summary(&outcome, args.games);
        update_stats(&mut stats, &outcome);
    }

    print_final_summary(&args, &stats);

    let errors = stats.crashes + stats.illegal_moves + stats.assertion_failures;
    if errors > 0 {
        std::process::exit(1);
    }
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let mut engine = None;
    let mut openings = None;
    let mut games = None;
    let mut movetime = None;
    let mut max_moves = 300u32;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--engine" => {
                i += 1;
                engine = Some(args[i].clone());
            }
            "--openings" => {
                i += 1;
                openings = Some(args[i].clone());
            }
            "--games" => {
                i += 1;
                games = Some(args[i].parse::<u32>().expect("invalid games count"));
            }
            "--movetime" => {
                i += 1;
                movetime = Some(args[i].parse::<u64>().expect("invalid movetime"));
            }
            "--maxmoves" => {
                i += 1;
                max_moves = args[i].parse::<u32>().expect("invalid maxmoves");
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let engine = engine.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let openings = openings.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let games = games.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let movetime = movetime.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });

    Args {
        engine,
        openings,
        games,
        movetime,
        max_moves,
    }
}

fn print_usage() {
    eprintln!(
        "Usage: selfplay --engine <path> --openings <path> --games <N> --movetime <ms> [--maxmoves <N>]"
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

struct EngineHandle {
    child: Child,
    writer: BufWriter<std::process::ChildStdin>,
    rx: mpsc::Receiver<String>,
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
    engine_path: &str,
    opening_fen: &str,
    game_num: u32,
    timeout: &Duration,
    max_moves: u32,
) -> GameOutcome {
    let mut pos = Position::from_fen(opening_fen).unwrap_or_else(|e| {
        panic!("invalid opening FEN '{opening_fen}': {e:?}");
    });

    let mut hash_history: Vec<u64> = vec![pos.hash()];
    let mut move_history: Vec<String> = Vec::new();
    let mut ply_count = 0u32;

    let mut white_engine = spawn_engine(engine_path);
    let mut black_engine = spawn_engine(engine_path);

    let result = loop {
        if ply_count >= max_moves {
            break GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: Some(GameResult::Draw),
                reason: GameEndReason::MaxMovesReached,
                ply_count,
            };
        }

        let side = pos.side_to_move();
        let side_name: &'static str = if side == Color::White {
            "white"
        } else {
            "black"
        };
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

        let go_cmd = format!("go movetime {}", timeout.as_millis() / 10);
        send_line(&mut engine.writer, &go_cmd);

        let bestmove_str = match wait_for_bestmove(&engine.rx, *timeout) {
            Ok(s) => s,
            Err(_) => {
                break GameOutcome {
                    game_num,
                    opening_fen: opening_fen.to_string(),
                    result: None,
                    reason: GameEndReason::TimeLoss { side: side_name },
                    ply_count,
                };
            }
        };

        if bestmove_str == "0000" || bestmove_str == "(none)" {
            let result = if side == Color::White {
                Some(GameResult::BlackWins)
            } else {
                Some(GameResult::WhiteWins)
            };
            break GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result,
                reason: GameEndReason::EngineResigned { side: side_name },
                ply_count,
            };
        }

        let mv = match validate_move(&mut pos, &bestmove_str) {
            Some(mv) => mv,
            None => {
                break GameOutcome {
                    game_num,
                    opening_fen: opening_fen.to_string(),
                    result: None,
                    reason: GameEndReason::IllegalMove {
                        side: side_name,
                        move_str: bestmove_str,
                    },
                    ply_count,
                };
            }
        };

        let _undo = pos.make_move(mv);
        move_history.push(bestmove_str);
        ply_count += 1;
        hash_history.push(pos.hash());

        let legal_moves = generate_legal_moves(&mut pos);
        if legal_moves.is_empty() {
            if is_in_check(&pos) {
                let result = if pos.side_to_move() == Color::White {
                    GameResult::BlackWins
                } else {
                    GameResult::WhiteWins
                };
                break GameOutcome {
                    game_num,
                    opening_fen: opening_fen.to_string(),
                    result: Some(result),
                    reason: GameEndReason::Checkmate,
                    ply_count,
                };
            } else {
                break GameOutcome {
                    game_num,
                    opening_fen: opening_fen.to_string(),
                    result: Some(GameResult::Draw),
                    reason: GameEndReason::Stalemate,
                    ply_count,
                };
            }
        }

        if pos.halfmove_clock() >= 100 {
            break GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: Some(GameResult::Draw),
                reason: GameEndReason::FiftyMoveRule,
                ply_count,
            };
        }

        if count_repetitions(&hash_history, pos.hash()) >= 3 {
            break GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: Some(GameResult::Draw),
                reason: GameEndReason::ThreefoldRepetition,
                ply_count,
            };
        }

        if is_insufficient_material(&pos) {
            break GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: Some(GameResult::Draw),
                reason: GameEndReason::InsufficientMaterial,
                ply_count,
            };
        }
    };

    send_line(&mut white_engine.writer, "quit");
    send_line(&mut black_engine.writer, "quit");

    let white_status = white_engine.child.wait().ok().and_then(|s| s.code());
    let black_status = black_engine.child.wait().ok().and_then(|s| s.code());

    if let Some(code) = white_status {
        if code != 0 {
            return GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: None,
                reason: GameEndReason::Crash {
                    side: "white",
                    exit_status: Some(code),
                },
                ply_count,
            };
        }
    }
    if let Some(code) = black_status {
        if code != 0 {
            return GameOutcome {
                game_num,
                opening_fen: opening_fen.to_string(),
                result: None,
                reason: GameEndReason::Crash {
                    side: "black",
                    exit_status: Some(code),
                },
                ply_count,
            };
        }
    }

    result
}

fn print_game_summary(outcome: &GameOutcome, total_games: u32) {
    let game_display = outcome.game_num + 1;
    let result_str = match &outcome.result {
        Some(GameResult::WhiteWins) => "1-0",
        Some(GameResult::BlackWins) => "0-1",
        Some(GameResult::Draw) => "1/2-1/2",
        None => "ERROR",
    };

    let reason_str = match &outcome.reason {
        GameEndReason::Checkmate => "checkmate".to_string(),
        GameEndReason::Stalemate => "stalemate".to_string(),
        GameEndReason::FiftyMoveRule => "fifty-move rule".to_string(),
        GameEndReason::ThreefoldRepetition => "threefold repetition".to_string(),
        GameEndReason::InsufficientMaterial => "insufficient material".to_string(),
        GameEndReason::MaxMovesReached => "max moves reached".to_string(),
        GameEndReason::Crash { side, exit_status } => {
            format!("crash by {side} (exit: {exit_status:?})")
        }
        GameEndReason::IllegalMove { side, move_str } => {
            format!("illegal move by {side}: \"{move_str}\"")
        }
        GameEndReason::TimeLoss { side } => format!("time loss by {side}"),
        GameEndReason::EngineResigned { side } => format!("{side} resigned"),
    };

    let fen_short = if outcome.opening_fen.len() > 40 {
        format!("{}...", &outcome.opening_fen[..40])
    } else {
        outcome.opening_fen.clone()
    };

    if outcome.result.is_none() {
        println!("Game {game_display}/{total_games}: {result_str} ({reason_str}) [{fen_short}]");
    } else {
        println!(
            "Game {game_display}/{total_games}: {result_str} ({reason_str}, {} ply) [{fen_short}]",
            outcome.ply_count
        );
    }
}

fn update_stats(stats: &mut Stats, outcome: &GameOutcome) {
    stats.total_games += 1;
    match &outcome.reason {
        GameEndReason::Crash { exit_status, .. } => {
            stats.crashes += 1;
            if exit_status.is_some() && exit_status.unwrap() != 0 {
                stats.assertion_failures += 1;
            }
        }
        GameEndReason::IllegalMove { .. } => {
            stats.illegal_moves += 1;
        }
        GameEndReason::TimeLoss { .. } => {
            stats.time_losses += 1;
        }
        _ => {}
    }
    match &outcome.result {
        Some(GameResult::WhiteWins) => stats.white_wins += 1,
        Some(GameResult::BlackWins) => stats.black_wins += 1,
        Some(GameResult::Draw) => stats.draws += 1,
        None => {}
    }
}

fn print_final_summary(args: &Args, stats: &Stats) {
    println!();
    println!("=== Self-Play Results ===");
    println!("Engine: {}", args.engine);
    println!("Games: {}", stats.total_games);
    println!("Time control: {}ms/move", args.movetime);
    println!();
    println!(
        "Results: +{} ={} -{} (W/D/L from white's perspective)",
        stats.white_wins, stats.draws, stats.black_wins
    );
    println!();
    println!("Errors:");
    println!("  Crashes:            {}", stats.crashes);
    println!("  Assertion failures: {}", stats.assertion_failures);
    println!("  Illegal moves:      {}", stats.illegal_moves);
    println!("  Time losses:        {}", stats.time_losses);
    println!();

    let critical_errors = stats.crashes + stats.assertion_failures + stats.illegal_moves;
    if critical_errors == 0 {
        println!("Status: PASS (0 crashes, 0 assertion failures, 0 illegal moves)");
    } else {
        println!(
            "Status: FAIL ({} crashes, {} assertion failures, {} illegal moves)",
            stats.crashes, stats.assertion_failures, stats.illegal_moves
        );
    }
}
