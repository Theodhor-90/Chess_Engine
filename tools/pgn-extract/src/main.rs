use chess_board::Position;
use chess_movegen::generate_legal_moves;
use chess_types::{File, PieceKind, Rank, Square};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};

fn char_to_file(ch: u8) -> Option<File> {
    match ch {
        b'a' => Some(File::A),
        b'b' => Some(File::B),
        b'c' => Some(File::C),
        b'd' => Some(File::D),
        b'e' => Some(File::E),
        b'f' => Some(File::F),
        b'g' => Some(File::G),
        b'h' => Some(File::H),
        _ => None,
    }
}

fn char_to_rank(ch: u8) -> Option<Rank> {
    match ch {
        b'1' => Some(Rank::R1),
        b'2' => Some(Rank::R2),
        b'3' => Some(Rank::R3),
        b'4' => Some(Rank::R4),
        b'5' => Some(Rank::R5),
        b'6' => Some(Rank::R6),
        b'7' => Some(Rank::R7),
        b'8' => Some(Rank::R8),
        _ => None,
    }
}

fn char_to_piece_kind(ch: u8) -> Option<PieceKind> {
    match ch {
        b'N' => Some(PieceKind::Knight),
        b'B' => Some(PieceKind::Bishop),
        b'R' => Some(PieceKind::Rook),
        b'Q' => Some(PieceKind::Queen),
        b'K' => Some(PieceKind::King),
        _ => None,
    }
}

fn char_to_promotion(ch: u8) -> Option<PieceKind> {
    match ch {
        b'N' | b'n' => Some(PieceKind::Knight),
        b'B' | b'b' => Some(PieceKind::Bishop),
        b'R' | b'r' => Some(PieceKind::Rook),
        b'Q' | b'q' => Some(PieceKind::Queen),
        _ => None,
    }
}

struct SanInfo {
    piece: PieceKind,
    dest_file: Option<File>,
    dest_rank: Option<Rank>,
    disambig_file: Option<File>,
    disambig_rank: Option<Rank>,
    promotion: Option<PieceKind>,
    _is_capture: bool,
    is_kingside_castle: bool,
    is_queenside_castle: bool,
}

fn parse_san(san: &str) -> Option<SanInfo> {
    let san = san.trim_end_matches('+').trim_end_matches('#');
    if san.is_empty() {
        return None;
    }

    if san == "O-O-O" || san == "0-0-0" {
        return Some(SanInfo {
            piece: PieceKind::King,
            dest_file: None,
            dest_rank: None,
            disambig_file: None,
            disambig_rank: None,
            promotion: None,
            _is_capture: false,
            is_kingside_castle: false,
            is_queenside_castle: true,
        });
    }
    if san == "O-O" || san == "0-0" {
        return Some(SanInfo {
            piece: PieceKind::King,
            dest_file: None,
            dest_rank: None,
            disambig_file: None,
            disambig_rank: None,
            promotion: None,
            _is_capture: false,
            is_kingside_castle: true,
            is_queenside_castle: false,
        });
    }

    let bytes = san.as_bytes();
    let mut idx = 0;

    let piece = if let Some(pk) = char_to_piece_kind(bytes[idx]) {
        idx += 1;
        pk
    } else {
        PieceKind::Pawn
    };

    let mut disambig_file: Option<File> = None;
    let mut disambig_rank: Option<Rank> = None;
    let mut is_capture = false;
    let mut dest_file: Option<File> = None;
    let mut dest_rank: Option<Rank> = None;
    let mut promotion: Option<PieceKind> = None;

    // Collect remaining characters, stripping 'x'
    let mut chars: Vec<u8> = Vec::new();
    while idx < bytes.len() {
        let ch = bytes[idx];
        if ch == b'x' {
            is_capture = true;
        } else if ch == b'=' {
            // promotion follows
            idx += 1;
            if idx < bytes.len() {
                promotion = char_to_promotion(bytes[idx]);
            }
            idx += 1;
            continue;
        } else {
            chars.push(ch);
        }
        idx += 1;
    }

    // Check if last char is a promotion piece without '=' prefix (e.g., "e8Q")
    if promotion.is_none() && !chars.is_empty() {
        if let Some(pk) = char_to_promotion(chars[chars.len() - 1]) {
            if piece == PieceKind::Pawn {
                promotion = Some(pk);
                chars.pop();
            }
        }
    }

    // Now parse the remaining chars to find dest square and optional disambiguation
    // The destination square is always the last file+rank pair
    let len = chars.len();
    if len >= 2 {
        if let (Some(f), Some(r)) = (char_to_file(chars[len - 2]), char_to_rank(chars[len - 1])) {
            dest_file = Some(f);
            dest_rank = Some(r);

            // Check for disambiguation before the destination
            if len >= 4 {
                // Could be file+rank disambiguation
                if let (Some(df), Some(dr)) =
                    (char_to_file(chars[len - 4]), char_to_rank(chars[len - 3]))
                {
                    disambig_file = Some(df);
                    disambig_rank = Some(dr);
                }
            }
            if disambig_file.is_none() && len >= 3 {
                // Could be file-only or rank-only disambiguation
                if let Some(df) = char_to_file(chars[len - 3]) {
                    disambig_file = Some(df);
                } else if let Some(dr) = char_to_rank(chars[len - 3]) {
                    disambig_rank = Some(dr);
                }
            }
        }
    }

    dest_file?;

    Some(SanInfo {
        piece,
        dest_file,
        dest_rank,
        disambig_file,
        disambig_rank,
        promotion,
        _is_capture: is_capture,
        is_kingside_castle: false,
        is_queenside_castle: false,
    })
}

fn match_san_to_move(pos: &mut Position, san: &str) -> Option<chess_types::Move> {
    let info = parse_san(san)?;

    if info.is_kingside_castle {
        let moves = generate_legal_moves(pos);
        return moves.into_iter().find(|m| {
            m.is_castling() && {
                let to = m.to_sq();
                to.file() == File::G
            }
        });
    }
    if info.is_queenside_castle {
        let moves = generate_legal_moves(pos);
        return moves.into_iter().find(|m| {
            m.is_castling() && {
                let to = m.to_sq();
                to.file() == File::C
            }
        });
    }

    let moves = generate_legal_moves(pos);
    let dest_file = info.dest_file?;
    let dest_rank = info.dest_rank?;
    let dest_sq = Square::from_file_rank(dest_file, dest_rank);

    for mv in &moves {
        if mv.to_sq() != dest_sq {
            continue;
        }

        let from_sq = mv.from_sq();
        let moving_piece = pos.piece_on(from_sq)?;
        if moving_piece.kind != info.piece {
            continue;
        }

        if let Some(df) = info.disambig_file {
            if from_sq.file() != df {
                continue;
            }
        }
        if let Some(dr) = info.disambig_rank {
            if from_sq.rank() != dr {
                continue;
            }
        }

        if let Some(promo) = info.promotion {
            if mv.promotion_piece() != Some(promo) {
                continue;
            }
        } else if mv.is_promotion() {
            continue;
        }

        return Some(*mv);
    }
    None
}

struct PgnGame {
    result: Option<f64>,
    start_fen: Option<String>,
    moves: Vec<String>,
}

fn extract_san_tokens(movetext: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for token in movetext.split_whitespace() {
        // Skip move numbers like "1." or "1..."
        if token.ends_with('.') || token.contains("...") {
            continue;
        }
        // Skip if it's purely numeric (move number without dot)
        if token.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // Skip result markers
        if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
            continue;
        }
        // Skip NAGs ($1, $2, etc.)
        if token.starts_with('$') {
            continue;
        }
        // Skip comments in braces (already handled by caller, but just in case)
        if token.starts_with('{') || token.ends_with('}') {
            continue;
        }
        tokens.push(token.to_string());
    }
    tokens
}

fn parse_pgn_games(path: &str) -> Vec<PgnGame> {
    let file = fs::File::open(path).unwrap_or_else(|e| {
        eprintln!("Failed to open PGN file: {e}");
        std::process::exit(1);
    });
    let reader = io::BufReader::new(file);

    let mut games = Vec::new();
    let mut current_result: Option<f64> = None;
    let mut current_fen: Option<String> = None;
    let mut movetext = String::new();
    let mut in_headers = true;
    let mut has_game = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !in_headers && has_game {
                // End of movetext block - strip comments from movetext
                let cleaned = strip_comments(&movetext);
                let move_tokens = extract_san_tokens(&cleaned);
                games.push(PgnGame {
                    result: current_result,
                    start_fen: current_fen.take(),
                    moves: move_tokens,
                });
                movetext.clear();
                current_result = None;
                in_headers = true;
                has_game = false;
            }
            continue;
        }

        if trimmed.starts_with('[') {
            in_headers = true;
            has_game = true;
            if trimmed.starts_with("[Result ") {
                current_result = parse_result_tag(trimmed);
            } else if trimmed.starts_with("[FEN ") {
                current_fen = parse_tag_value(trimmed);
            }
        } else {
            in_headers = false;
            has_game = true;
            if !movetext.is_empty() {
                movetext.push(' ');
            }
            movetext.push_str(trimmed);
        }
    }

    // Handle last game
    if !movetext.is_empty() && has_game {
        let cleaned = strip_comments(&movetext);
        let move_tokens = extract_san_tokens(&cleaned);
        games.push(PgnGame {
            result: current_result,
            start_fen: current_fen,
            moves: move_tokens,
        });
    }

    games
}

fn strip_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth = 0;
    let mut in_semicolon = false;
    for ch in s.chars() {
        if in_semicolon {
            if ch == '\n' {
                in_semicolon = false;
            }
            continue;
        }
        if ch == ';' && depth == 0 {
            in_semicolon = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
            continue;
        }
        if ch == '}' {
            if depth > 0 {
                depth -= 1;
            }
            continue;
        }
        if depth == 0 {
            // Also strip RAV (recursive annotation variation) parentheses
            if ch == '(' {
                depth += 1;
                continue;
            }
            result.push(ch);
        }
    }
    result
}

fn parse_result_tag(tag: &str) -> Option<f64> {
    let val = parse_tag_value(tag)?;
    match val.as_str() {
        "1-0" => Some(1.0),
        "0-1" => Some(0.0),
        "1/2-1/2" => Some(0.5),
        _ => None,
    }
}

fn parse_tag_value(tag: &str) -> Option<String> {
    let start = tag.find('"')? + 1;
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn is_quiet_position(pos: &mut Position) -> bool {
    let side = pos.side_to_move();
    let king_bb = pos.piece_bitboard(chess_types::Piece::new(side, PieceKind::King));
    let king_sq = match king_bb.into_iter().next() {
        Some(sq) => Square::new(sq as u8).unwrap(),
        None => return false,
    };

    if pos.is_square_attacked(king_sq, side.opposite()) {
        return false;
    }

    let moves = generate_legal_moves(pos);
    for mv in &moves {
        if mv.is_capture() {
            return false;
        }
    }
    true
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut pgn_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut max_positions: Option<usize> = None;
    let mut skip_first: usize = 10;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pgn" => {
                i += 1;
                if i < args.len() {
                    pgn_path = Some(args[i].clone());
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(args[i].clone());
                }
            }
            "--max-positions" => {
                i += 1;
                if i < args.len() {
                    max_positions = Some(args[i].parse().expect("invalid max-positions value"));
                }
            }
            "--skip-first" => {
                i += 1;
                if i < args.len() {
                    skip_first = args[i].parse().expect("invalid skip-first value");
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let pgn_path = pgn_path.expect("--pgn <path> is required");
    let output_path = output_path.expect("--output <path> is required");

    eprintln!("Parsing PGN file: {pgn_path}");
    let games = parse_pgn_games(&pgn_path);
    eprintln!("Parsed {} games", games.len());

    let out_file = fs::File::create(&output_path).unwrap_or_else(|e| {
        eprintln!("Failed to create output file: {e}");
        std::process::exit(1);
    });
    let mut writer = io::BufWriter::new(out_file);

    let mut total_positions = 0usize;
    let mut games_processed = 0usize;

    for game in &games {
        let result = match game.result {
            Some(r) => r,
            None => continue,
        };

        let mut pos = if let Some(ref fen) = game.start_fen {
            match Position::from_fen(fen) {
                Ok(p) => p,
                Err(_) => continue,
            }
        } else {
            Position::startpos()
        };

        let mut ply = 0usize;
        let mut game_ok = true;

        for san in &game.moves {
            let mv = match match_san_to_move(&mut pos, san) {
                Some(m) => m,
                None => {
                    game_ok = false;
                    break;
                }
            };

            pos.make_move(mv);
            ply += 1;

            if ply > skip_first && is_quiet_position(&mut pos) {
                let fen = pos.to_fen();
                let result_str = if result == 1.0 {
                    "1.0"
                } else if result == 0.0 {
                    "0.0"
                } else {
                    "0.5"
                };
                writeln!(writer, "{fen}|{result_str}").unwrap();
                total_positions += 1;

                if let Some(max) = max_positions {
                    if total_positions >= max {
                        break;
                    }
                }
            }
        }

        if let Some(max) = max_positions {
            if total_positions >= max {
                break;
            }
        }

        games_processed += 1;
        if !game_ok {
            continue;
        }

        if games_processed.is_multiple_of(10_000) {
            eprintln!("Processed {games_processed} games, {total_positions} positions extracted");
        }
    }

    writer.flush().unwrap();
    eprintln!(
        "Done: {games_processed} games processed, {total_positions} positions extracted to {output_path}"
    );
}
