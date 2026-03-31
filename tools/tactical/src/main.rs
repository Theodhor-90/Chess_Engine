use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use chess_board::Position;
use chess_movegen::generate_legal_moves;
use chess_types::{File, MoveFlag, PieceKind, Rank, Square};

struct EpdEntry {
    fen: String,
    best_moves_san: Vec<String>,
    id: String,
}

enum SearchMode {
    Depth(u32),
    MoveTime(u64),
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut engine_path = None;
    let mut suite_paths: Vec<String> = Vec::new();
    let mut search_mode = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--engine" => {
                i += 1;
                engine_path = Some(args[i].clone());
            }
            "--suite" => {
                i += 1;
                suite_paths.push(args[i].clone());
            }
            "--depth" => {
                i += 1;
                let d: u32 = args[i].parse().expect("invalid depth");
                search_mode = Some(SearchMode::Depth(d));
            }
            "--movetime" => {
                i += 1;
                let t: u64 = args[i].parse().expect("invalid movetime");
                search_mode = Some(SearchMode::MoveTime(t));
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let engine_path = engine_path.expect("--engine <path> is required");
    if suite_paths.is_empty() {
        eprintln!("--suite <epd_file> is required (can be specified multiple times)");
        std::process::exit(1);
    }
    let search_mode = search_mode.expect("either --depth <N> or --movetime <ms> is required");

    let mode_str = match &search_mode {
        SearchMode::Depth(d) => format!("depth {d}"),
        SearchMode::MoveTime(t) => format!("movetime {t}ms"),
    };

    println!("=== Tactical Test Suite ===");
    println!("Engine: {engine_path}");
    println!("Mode: {mode_str}");
    println!();

    let mut total_solved = 0u32;
    let mut total_positions = 0u32;

    for suite_path in &suite_paths {
        let entries = parse_epd(suite_path);
        let suite_name = Path::new(suite_path).file_name().unwrap().to_string_lossy();
        let count = entries.len();

        println!("--- {suite_name} ({count} positions) ---");

        let mut child = Command::new(&engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn engine");

        let mut engine_in = child.stdin.take().expect("failed to open engine stdin");
        let engine_out = child.stdout.take().expect("failed to open engine stdout");
        let mut reader = BufReader::new(engine_out);

        send_line(&mut engine_in, "uci");
        wait_for(&mut reader, "uciok");

        let mut solved = 0u32;

        for (idx, entry) in entries.iter().enumerate() {
            send_line(&mut engine_in, "isready");
            wait_for(&mut reader, "readyok");

            let pos_cmd = format!("position fen {}", entry.fen);
            send_line(&mut engine_in, &pos_cmd);

            let go_cmd = match &search_mode {
                SearchMode::Depth(d) => format!("go depth {d}"),
                SearchMode::MoveTime(t) => format!("go movetime {t}"),
            };
            send_line(&mut engine_in, &go_cmd);

            let bestmove_uci = wait_for_bestmove(&mut reader);

            let expected_uci = resolve_san_moves(&entry.fen, &entry.best_moves_san);

            let pass = expected_uci.contains(&bestmove_uci);
            if pass {
                solved += 1;
            }

            let expected_san = entry.best_moves_san.join("/");
            let status = if pass { "PASS" } else { "FAIL" };
            let id_str = if entry.id.is_empty() {
                String::new()
            } else {
                format!(" {}", entry.id)
            };
            println!(
                "  [{}/{}]{}: expected {}, got {} ... {}",
                idx + 1,
                count,
                id_str,
                expected_san,
                bestmove_uci,
                status
            );
        }

        send_line(&mut engine_in, "quit");
        let _ = child.wait();

        let pct = if count > 0 {
            (solved as f64 / count as f64) * 100.0
        } else {
            0.0
        };
        println!("Suite: {suite_name} — Solved: {solved}/{count} ({pct:.1}%)");
        println!();

        total_solved += solved;
        total_positions += count as u32;
    }

    let total_pct = if total_positions > 0 {
        (total_solved as f64 / total_positions as f64) * 100.0
    } else {
        0.0
    };
    println!("=== Summary ===");
    println!("Total solved: {total_solved}/{total_positions} ({total_pct:.1}%)");
}

fn parse_epd(path: &str) -> Vec<EpdEntry> {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Failed to read {path}: {e}");
        std::process::exit(1);
    });
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        if parts.len() < 5 {
            continue;
        }

        let fen = format!("{} {} {} {} 0 1", parts[0], parts[1], parts[2], parts[3]);

        let remainder = parts[4];

        let best_moves = parse_bm_field(remainder);
        if best_moves.is_empty() {
            continue;
        }

        let id = parse_id_field(remainder);

        entries.push(EpdEntry {
            fen,
            best_moves_san: best_moves,
            id,
        });
    }

    entries
}

fn parse_bm_field(remainder: &str) -> Vec<String> {
    let mut moves = Vec::new();
    if let Some(bm_start) = remainder.find("bm ") {
        let after_bm = &remainder[bm_start + 3..];
        let end = after_bm.find(';').unwrap_or(after_bm.len());
        let bm_str = after_bm[..end].trim();
        for m in bm_str.split_whitespace() {
            moves.push(m.to_string());
        }
    }
    moves
}

fn parse_id_field(remainder: &str) -> String {
    if let Some(id_start) = remainder.find("id \"") {
        let after_id = &remainder[id_start + 4..];
        if let Some(end) = after_id.find('"') {
            return after_id[..end].to_string();
        }
    }
    String::new()
}

fn resolve_san_moves(fen: &str, sans: &[String]) -> Vec<String> {
    let mut pos = match Position::from_fen(fen) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let legal_moves = generate_legal_moves(&mut pos);
    let mut uci_moves = Vec::new();

    for san in sans {
        match resolve_san(&pos, &legal_moves, san) {
            Some(mv) => uci_moves.push(format!("{mv}")),
            None => {
                eprintln!("  WARNING: could not resolve SAN '{san}' in position {fen}");
            }
        }
    }

    uci_moves
}

fn resolve_san(
    pos: &Position,
    legal_moves: &[chess_types::Move],
    san: &str,
) -> Option<chess_types::Move> {
    let san = strip_decorations(san);

    if let Some(mv) = resolve_castling(pos, legal_moves, &san) {
        return Some(mv);
    }

    let parsed = parse_san_components(&san)?;

    let candidates: Vec<chess_types::Move> = legal_moves
        .iter()
        .copied()
        .filter(|mv| {
            let from = mv.from_sq();
            let to = mv.to_sq();

            if to != parsed.target_sq {
                return false;
            }

            let piece = pos.piece_on(from);
            let piece_kind = match piece {
                Some(p) => p.kind,
                None => return false,
            };

            if piece_kind != parsed.piece {
                return false;
            }

            if let Some(promo) = parsed.promotion {
                match mv.promotion_piece() {
                    Some(pk) if pk == promo => {}
                    _ => return false,
                }
            } else if mv.is_promotion() {
                return false;
            }

            if let Some(file) = parsed.disambig_file {
                if from.file() != file {
                    return false;
                }
            }

            if let Some(rank) = parsed.disambig_rank {
                if from.rank() != rank {
                    return false;
                }
            }

            true
        })
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        if candidates.len() > 1 {
            eprintln!(
                "  WARNING: ambiguous SAN '{san}' — {len} matches",
                len = candidates.len()
            );
        }
        None
    }
}

fn strip_decorations(san: &str) -> String {
    san.chars()
        .filter(|c| !matches!(c, '+' | '#' | '!' | '?'))
        .collect()
}

fn resolve_castling(
    pos: &Position,
    legal_moves: &[chess_types::Move],
    san: &str,
) -> Option<chess_types::Move> {
    let flag = if san == "O-O-O" || san == "0-0-0" {
        MoveFlag::QUEEN_CASTLE
    } else if san == "O-O" || san == "0-0" {
        MoveFlag::KING_CASTLE
    } else {
        return None;
    };

    let king_sq = find_king_square(pos);
    legal_moves
        .iter()
        .copied()
        .find(|mv| mv.flags() == flag && mv.from_sq() == king_sq)
}

fn find_king_square(pos: &Position) -> Square {
    let side = pos.side_to_move();
    let king = chess_types::Piece::new(side, PieceKind::King);
    let bb = pos.piece_bitboard(king);
    for i in 0..64 {
        let sq = Square::new(i).unwrap();
        if !bb.is_empty() && pos.piece_on(sq) == Some(king) {
            return sq;
        }
    }
    panic!("no king found");
}

struct SanComponents {
    piece: PieceKind,
    target_sq: Square,
    disambig_file: Option<File>,
    disambig_rank: Option<Rank>,
    promotion: Option<PieceKind>,
}

fn parse_san_components(san: &str) -> Option<SanComponents> {
    let chars: Vec<char> = san.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut idx = 0;

    let piece = if chars[0].is_uppercase() && "KQRBN".contains(chars[0]) {
        let p = char_to_piece_kind(chars[0])?;
        idx += 1;
        p
    } else {
        PieceKind::Pawn
    };

    let mut disambig_file: Option<File> = None;
    let mut disambig_rank: Option<Rank> = None;
    let mut target_file: Option<File> = None;
    let mut target_rank: Option<Rank> = None;

    let remaining = &chars[idx..];

    let mut files_and_ranks: Vec<(Option<File>, Option<Rank>)> = Vec::new();
    let mut ri = 0;
    while ri < remaining.len() {
        if remaining[ri] == 'x' {
            ri += 1;
            continue;
        }
        if remaining[ri] == '=' {
            break;
        }
        if remaining[ri].is_ascii_lowercase() && ('a'..='h').contains(&remaining[ri]) {
            let f = char_to_file(remaining[ri])?;
            if ri + 1 < remaining.len()
                && remaining[ri + 1].is_ascii_digit()
                && ('1'..='8').contains(&remaining[ri + 1])
            {
                let r = char_to_rank(remaining[ri + 1])?;
                files_and_ranks.push((Some(f), Some(r)));
                ri += 2;
            } else {
                files_and_ranks.push((Some(f), None));
                ri += 1;
            }
        } else if remaining[ri].is_ascii_digit() && ('1'..='8').contains(&remaining[ri]) {
            let r = char_to_rank(remaining[ri])?;
            files_and_ranks.push((None, Some(r)));
            ri += 1;
        } else {
            break;
        }
    }

    match files_and_ranks.len() {
        0 => return None,
        1 => {
            let (f, r) = files_and_ranks[0];
            if f.is_some() && r.is_some() {
                target_file = f;
                target_rank = r;
            } else if piece == PieceKind::Pawn && f.is_some() && r.is_none() {
                disambig_file = f;
            } else {
                return None;
            }
        }
        2 => {
            let (f2, r2) = files_and_ranks[1];
            if f2.is_some() && r2.is_some() {
                target_file = f2;
                target_rank = r2;
                let (f1, r1) = files_and_ranks[0];
                disambig_file = f1;
                disambig_rank = r1;
            } else if piece == PieceKind::Pawn {
                let (f1, r1) = files_and_ranks[0];
                disambig_file = f1;
                disambig_rank = r1;
                target_file = f2;
                target_rank = r2;
            } else {
                return None;
            }
        }
        3 => {
            let (f1, r1) = files_and_ranks[0];
            disambig_file = f1;
            disambig_rank = r1;
            let (f2, _) = files_and_ranks[1];
            let (_, r3) = files_and_ranks[2];
            if f2.is_some() && r3.is_some() {
                target_file = f2;
                target_rank = r3;
            } else {
                return None;
            }
        }
        _ => return None,
    }

    let promotion = {
        let eq_pos = remaining.iter().position(|&c| c == '=');
        if let Some(ep) = eq_pos {
            if ep + 1 < remaining.len() {
                char_to_piece_kind(remaining[ep + 1])
            } else {
                None
            }
        } else {
            None
        }
    };

    let target_sq = if let (Some(f), Some(r)) = (target_file, target_rank) {
        Square::from_file_rank(f, r)
    } else {
        return None;
    };

    Some(SanComponents {
        piece,
        target_sq,
        disambig_file,
        disambig_rank,
        promotion,
    })
}

fn char_to_piece_kind(c: char) -> Option<PieceKind> {
    match c {
        'K' => Some(PieceKind::King),
        'Q' => Some(PieceKind::Queen),
        'R' => Some(PieceKind::Rook),
        'B' => Some(PieceKind::Bishop),
        'N' => Some(PieceKind::Knight),
        _ => None,
    }
}

fn char_to_file(c: char) -> Option<File> {
    let idx = (c as u8).wrapping_sub(b'a');
    File::try_from(idx).ok()
}

fn char_to_rank(c: char) -> Option<Rank> {
    let idx = (c as u8).wrapping_sub(b'1');
    Rank::try_from(idx).ok()
}

fn send_line(stdin: &mut impl Write, line: &str) {
    writeln!(stdin, "{line}").expect("failed to write to engine");
    stdin.flush().expect("failed to flush engine stdin");
}

fn wait_for(reader: &mut impl BufRead, expected: &str) {
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .expect("failed to read from engine");
        if n == 0 {
            panic!("engine closed stdout while waiting for '{expected}'");
        }
        if buf.trim() == expected {
            return;
        }
    }
}

fn wait_for_bestmove(reader: &mut impl BufRead) -> String {
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .expect("failed to read from engine");
        if n == 0 {
            panic!("engine closed stdout while waiting for bestmove");
        }
        let trimmed = buf.trim();
        if let Some(rest) = trimmed.strip_prefix("bestmove") {
            let rest = rest.trim();
            let mv = rest.split_whitespace().next().unwrap_or("0000");
            return mv.to_string();
        }
    }
}
