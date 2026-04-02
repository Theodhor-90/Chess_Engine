use std::cell::Cell;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;

use chess_board::Position;
use chess_movegen::generate_legal_moves;
use chess_types::{Color, Piece, PieceKind, Square};

struct Args {
    openings: String,
    games: u32,
    nodes: u64,
    output: String,
    max_moves: u32,
}

struct TrainingRecord {
    fen: String,
    eval: i32,
    result: f64,
}

enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

fn main() {
    let args = parse_args();
    let openings = load_openings(&args.openings);

    let mut total_positions: u64 = 0;

    for game_num in 0..args.games {
        let opening_index = game_num as usize % openings.len();
        let opening_fen = &openings[opening_index];

        let (records, result) = play_game(opening_fen, args.nodes, args.max_moves);

        let result_str = match &result {
            Some(GameResult::WhiteWins) => "1-0",
            Some(GameResult::BlackWins) => "0-1",
            Some(GameResult::Draw) => "1/2-1/2",
            None => "unknown",
        };

        let game_display = game_num + 1;
        println!(
            "Game {game_display}/{}: {result_str} ({} positions)",
            args.games,
            records.len()
        );

        total_positions += records.len() as u64;
        write_records(&args.output, &records);
    }

    println!();
    println!("=== Data Generation Summary ===");
    println!("Total games:    {}", args.games);
    println!("Total positions: {total_positions}");
    if args.games > 0 {
        println!(
            "Positions/game: {:.1}",
            total_positions as f64 / args.games as f64
        );
    }
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let mut openings = None;
    let mut games = None;
    let mut nodes = None;
    let mut output = None;
    let mut max_moves = 300u32;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--openings" => {
                i += 1;
                openings = Some(args[i].clone());
            }
            "--games" => {
                i += 1;
                games = Some(args[i].parse::<u32>().expect("invalid games count"));
            }
            "--nodes" => {
                i += 1;
                nodes = Some(args[i].parse::<u64>().expect("invalid nodes count"));
            }
            "--output" => {
                i += 1;
                output = Some(args[i].clone());
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

    let openings = openings.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let games = games.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let nodes = nodes.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });
    let output = output.unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });

    Args {
        openings,
        games,
        nodes,
        output,
        max_moves,
    }
}

fn print_usage() {
    eprintln!(
        "Usage: datagen --openings <path> --games <N> --nodes <N> --output <path> [--maxmoves <N>]"
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

fn play_game(
    opening_fen: &str,
    node_limit: u64,
    max_moves: u32,
) -> (Vec<TrainingRecord>, Option<GameResult>) {
    let mut pos = Position::from_fen(opening_fen).unwrap_or_else(|e| {
        panic!("invalid opening FEN '{opening_fen}': {e:?}");
    });

    let mut hash_history: Vec<u64> = vec![pos.hash()];
    let mut records: Vec<TrainingRecord> = Vec::new();
    let mut ply = 0u32;

    let result = loop {
        if ply >= max_moves {
            break Some(GameResult::Draw);
        }

        let legal_moves = generate_legal_moves(&mut pos);
        if legal_moves.is_empty() {
            if is_in_check(&pos) {
                let result = if pos.side_to_move() == Color::White {
                    GameResult::BlackWins
                } else {
                    GameResult::WhiteWins
                };
                break Some(result);
            } else {
                break Some(GameResult::Draw);
            }
        }

        if pos.halfmove_clock() >= 100 {
            break Some(GameResult::Draw);
        }

        if count_repetitions(&hash_history, pos.hash()) >= 3 {
            break Some(GameResult::Draw);
        }

        if is_insufficient_material(&pos) {
            break Some(GameResult::Draw);
        }

        let limits = chess_search::SearchLimits {
            max_time: Duration::from_secs(3600),
            max_depth: None,
            max_nodes: Some(node_limit),
            stop_flag: None,
        };

        let last_score: Cell<i32> = Cell::new(0);
        let callback: chess_search::DepthCallback<'_> = &|_depth, score, _nodes, _elapsed, _pv| {
            last_score.set(score);
        };
        let best_move =
            chess_search::search(&mut pos, limits, &hash_history, Some(&callback), None);

        let score = last_score.get();

        let mv = match best_move {
            Some(mv) => mv,
            None => break None,
        };

        records.push(TrainingRecord {
            fen: pos.to_fen(),
            eval: score,
            result: 0.0,
        });

        let _undo = pos.make_move(mv);
        ply += 1;
        hash_history.push(pos.hash());
    };

    let result_value = match &result {
        Some(GameResult::WhiteWins) => 1.0,
        Some(GameResult::BlackWins) => 0.0,
        Some(GameResult::Draw) => 0.5,
        None => 0.5,
    };

    for record in &mut records {
        record.result = result_value;
    }

    (records, result)
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

fn write_records(path: &str, records: &[TrainingRecord]) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open {path}: {e}");
            std::process::exit(1);
        });
    for r in records {
        writeln!(file, "{} | {} | {:.1}", r.fen, r.eval, r.result).expect("failed to write record");
    }
}
