use std::fmt::Write;

use chess_types::{Bitboard, Color, File, Piece, PieceKind, Rank, Square};
use thiserror::Error;

use crate::position::{CastlingRights, Position};

#[derive(Debug, Error)]
pub enum FenError {
    #[error("invalid FEN: expected 6 space-separated fields, got {0}")]
    WrongFieldCount(usize),
    #[error("invalid piece placement: {0}")]
    InvalidPiecePlacement(String),
    #[error("invalid active color: expected 'w' or 'b', got '{0}'")]
    InvalidActiveColor(String),
    #[error("invalid castling availability: '{0}'")]
    InvalidCastling(String),
    #[error("invalid en passant square: '{0}'")]
    InvalidEnPassant(String),
    #[error("invalid halfmove clock: '{0}'")]
    InvalidHalfmoveClock(String),
    #[error("invalid fullmove number: '{0}'")]
    InvalidFullmoveNumber(String),
}

fn char_to_piece(ch: char) -> Option<Piece> {
    match ch {
        'P' => Some(Piece::new(Color::White, PieceKind::Pawn)),
        'N' => Some(Piece::new(Color::White, PieceKind::Knight)),
        'B' => Some(Piece::new(Color::White, PieceKind::Bishop)),
        'R' => Some(Piece::new(Color::White, PieceKind::Rook)),
        'Q' => Some(Piece::new(Color::White, PieceKind::Queen)),
        'K' => Some(Piece::new(Color::White, PieceKind::King)),
        'p' => Some(Piece::new(Color::Black, PieceKind::Pawn)),
        'n' => Some(Piece::new(Color::Black, PieceKind::Knight)),
        'b' => Some(Piece::new(Color::Black, PieceKind::Bishop)),
        'r' => Some(Piece::new(Color::Black, PieceKind::Rook)),
        'q' => Some(Piece::new(Color::Black, PieceKind::Queen)),
        'k' => Some(Piece::new(Color::Black, PieceKind::King)),
        _ => None,
    }
}

pub(crate) fn parse_fen(fen: &str) -> Result<Position, FenError> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() != 6 {
        return Err(FenError::WrongFieldCount(parts.len()));
    }

    // Field 0: Piece placement
    let rank_strs: Vec<&str> = parts[0].split('/').collect();
    if rank_strs.len() != 8 {
        return Err(FenError::InvalidPiecePlacement("expected 8 ranks".into()));
    }

    let mut piece_bb = [Bitboard::EMPTY; 12];

    for (rank_str_idx, rank_str) in rank_strs.iter().enumerate() {
        let rank_idx = 7 - rank_str_idx;
        let rank = Rank::try_from(rank_idx as u8).map_err(|_| {
            FenError::InvalidPiecePlacement(format!("invalid rank index {rank_idx}"))
        })?;
        let mut file_idx: u8 = 0;

        for ch in rank_str.chars() {
            if let Some(digit) = ch.to_digit(10) {
                if (1..=8).contains(&digit) {
                    file_idx += digit as u8;
                } else {
                    return Err(FenError::InvalidPiecePlacement(format!(
                        "unexpected character '{ch}'"
                    )));
                }
            } else if let Some(piece) = char_to_piece(ch) {
                let file = File::try_from(file_idx).map_err(|_| {
                    FenError::InvalidPiecePlacement(format!(
                        "rank {} sums to more than 8 squares",
                        rank_idx + 1
                    ))
                })?;
                let sq = Square::from_file_rank(file, rank);
                piece_bb[piece.index()] |= Bitboard::new(1u64 << sq.index());
                file_idx += 1;
            } else {
                return Err(FenError::InvalidPiecePlacement(format!(
                    "unexpected character '{ch}'"
                )));
            }
        }

        if file_idx != 8 {
            return Err(FenError::InvalidPiecePlacement(format!(
                "rank {} sums to {file_idx} squares, expected 8",
                rank_idx + 1
            )));
        }
    }

    let mut white_occ = Bitboard::EMPTY;
    for bb in &piece_bb[..6] {
        white_occ |= *bb;
    }
    let mut black_occ = Bitboard::EMPTY;
    for bb in &piece_bb[6..] {
        black_occ |= *bb;
    }
    let occupied = white_occ | black_occ;
    let occupied_by = [white_occ, black_occ];

    // Field 1: Active color
    let side_to_move = match parts[1] {
        "w" => Color::White,
        "b" => Color::Black,
        other => return Err(FenError::InvalidActiveColor(other.to_string())),
    };

    // Field 2: Castling availability
    let mut castling_rights = CastlingRights::NONE;
    if parts[2] != "-" {
        for ch in parts[2].chars() {
            let flag = match ch {
                'K' => CastlingRights::WHITE_KINGSIDE,
                'Q' => CastlingRights::WHITE_QUEENSIDE,
                'k' => CastlingRights::BLACK_KINGSIDE,
                'q' => CastlingRights::BLACK_QUEENSIDE,
                _ => return Err(FenError::InvalidCastling(parts[2].to_string())),
            };
            if castling_rights.contains(flag) {
                return Err(FenError::InvalidCastling(parts[2].to_string()));
            }
            castling_rights.set(flag);
        }
    }

    // Field 3: En passant
    let en_passant = if parts[3] == "-" {
        None
    } else {
        let ep = parts[3];
        if ep.len() != 2 {
            return Err(FenError::InvalidEnPassant(ep.to_string()));
        }
        let mut chars = ep.chars();
        let file_char = chars.next().unwrap();
        let rank_char = chars.next().unwrap();

        if !file_char.is_ascii_lowercase() {
            return Err(FenError::InvalidEnPassant(ep.to_string()));
        }
        let file = File::try_from(file_char as u8 - b'a')
            .map_err(|_| FenError::InvalidEnPassant(ep.to_string()))?;
        let rank = Rank::try_from(rank_char as u8 - b'1')
            .map_err(|_| FenError::InvalidEnPassant(ep.to_string()))?;

        if !matches!(rank, Rank::R3 | Rank::R6) {
            return Err(FenError::InvalidEnPassant(ep.to_string()));
        }

        Some(Square::from_file_rank(file, rank))
    };

    // Field 4: Halfmove clock
    let halfmove_clock = parts[4]
        .parse::<u8>()
        .map_err(|_| FenError::InvalidHalfmoveClock(parts[4].to_string()))?;

    // Field 5: Fullmove number
    let fullmove_counter = parts[5]
        .parse::<u16>()
        .map_err(|_| FenError::InvalidFullmoveNumber(parts[5].to_string()))?;
    if fullmove_counter == 0 {
        return Err(FenError::InvalidFullmoveNumber(parts[5].to_string()));
    }

    Ok(Position::new(
        piece_bb,
        occupied_by,
        occupied,
        side_to_move,
        castling_rights,
        en_passant,
        halfmove_clock,
        fullmove_counter,
    ))
}

pub(crate) fn format_fen(pos: &Position) -> String {
    let mut result = String::new();

    // Piece placement
    for rank_idx in (0..8).rev() {
        let rank = Rank::try_from(rank_idx as u8).unwrap();
        let mut empty_count: u8 = 0;

        for file_idx in 0..8 {
            let file = File::try_from(file_idx as u8).unwrap();
            let sq = Square::from_file_rank(file, rank);

            if let Some(piece) = pos.piece_on(sq) {
                if empty_count > 0 {
                    result.push((b'0' + empty_count) as char);
                    empty_count = 0;
                }
                write!(result, "{piece}").unwrap();
            } else {
                empty_count += 1;
            }
        }

        if empty_count > 0 {
            result.push((b'0' + empty_count) as char);
        }

        if rank_idx > 0 {
            result.push('/');
        }
    }

    // Active color
    if pos.side_to_move() == Color::White {
        result.push_str(" w");
    } else {
        result.push_str(" b");
    }

    // Castling availability
    result.push(' ');
    let mut any_castling = false;
    if pos
        .castling_rights()
        .contains(CastlingRights::WHITE_KINGSIDE)
    {
        result.push('K');
        any_castling = true;
    }
    if pos
        .castling_rights()
        .contains(CastlingRights::WHITE_QUEENSIDE)
    {
        result.push('Q');
        any_castling = true;
    }
    if pos
        .castling_rights()
        .contains(CastlingRights::BLACK_KINGSIDE)
    {
        result.push('k');
        any_castling = true;
    }
    if pos
        .castling_rights()
        .contains(CastlingRights::BLACK_QUEENSIDE)
    {
        result.push('q');
        any_castling = true;
    }
    if !any_castling {
        result.push('-');
    }

    // En passant
    result.push(' ');
    if let Some(sq) = pos.en_passant() {
        write!(result, "{sq}").unwrap();
    } else {
        result.push('-');
    }

    // Halfmove clock
    write!(result, " {}", pos.halfmove_clock()).unwrap();

    // Fullmove number
    write!(result, " {}", pos.fullmove_counter()).unwrap();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_round_trip() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.to_fen(), fen);
        assert_eq!(pos.side_to_move(), Color::White);
        assert_eq!(pos.castling_rights(), CastlingRights::ALL);
        assert_eq!(pos.en_passant(), None);
        assert_eq!(pos.halfmove_clock(), 0);
        assert_eq!(pos.fullmove_counter(), 1);
    }

    #[test]
    fn middlegame_round_trips() {
        let fens = [
            (
                "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
                Color::White,
            ),
            (
                "r1bqkb1r/pppppppp/2n2n2/8/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
                Color::Black,
            ),
            (
                "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 2 4",
                Color::White,
            ),
        ];
        for (fen, expected_stm) in fens {
            let pos = Position::from_fen(fen).unwrap();
            assert_eq!(pos.to_fen(), fen);
            assert_eq!(pos.side_to_move(), expected_stm);
        }
    }

    #[test]
    fn endgame_round_trips() {
        let fens = [
            "8/8/4k3/8/8/4K3/4P3/8 w - - 0 1",
            "8/5k2/8/8/8/2K5/8/4R3 w - - 0 1",
        ];
        for fen in fens {
            let pos = Position::from_fen(fen).unwrap();
            assert_eq!(pos.to_fen(), fen);
            assert_eq!(pos.castling_rights(), CastlingRights::NONE);
            assert_eq!(pos.en_passant(), None);
        }
    }

    #[test]
    fn startpos_from_fen_matches_startpos() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let from_fen = Position::from_fen(fen).unwrap();
        let startpos = Position::startpos();

        for color in [Color::White, Color::Black] {
            for kind in [
                PieceKind::Pawn,
                PieceKind::Knight,
                PieceKind::Bishop,
                PieceKind::Rook,
                PieceKind::Queen,
                PieceKind::King,
            ] {
                let piece = Piece::new(color, kind);
                assert_eq!(
                    from_fen.piece_bitboard(piece),
                    startpos.piece_bitboard(piece),
                    "mismatch for {piece}"
                );
            }
            assert_eq!(from_fen.occupied_by(color), startpos.occupied_by(color));
        }
        assert_eq!(from_fen.occupied(), startpos.occupied());
        assert_eq!(from_fen.side_to_move(), startpos.side_to_move());
        assert_eq!(from_fen.castling_rights(), startpos.castling_rights());
        assert_eq!(from_fen.en_passant(), startpos.en_passant());
        assert_eq!(from_fen.halfmove_clock(), startpos.halfmove_clock());
        assert_eq!(from_fen.fullmove_counter(), startpos.fullmove_counter());
    }

    #[test]
    fn reject_wrong_field_count() {
        let result5 = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0");
        assert!(matches!(result5, Err(FenError::WrongFieldCount(5))));

        let result7 =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 extra");
        assert!(matches!(result7, Err(FenError::WrongFieldCount(7))));
    }

    #[test]
    fn reject_invalid_piece_placement() {
        // Rank with 9 squares
        let result =
            Position::from_fen("rnbqkbnrr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidPiecePlacement(_))));

        // Invalid piece character
        let result = Position::from_fen("xnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidPiecePlacement(_))));

        // Only 7 ranks
        let result = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidPiecePlacement(_))));
    }

    #[test]
    fn reject_invalid_active_color() {
        let result = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidActiveColor(_))));
    }

    #[test]
    fn reject_invalid_castling() {
        let result = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w X - 0 1");
        assert!(matches!(result, Err(FenError::InvalidCastling(_))));
    }

    #[test]
    fn reject_invalid_en_passant() {
        let result =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e9 0 1");
        assert!(matches!(result, Err(FenError::InvalidEnPassant(_))));

        let result =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq z3 0 1");
        assert!(matches!(result, Err(FenError::InvalidEnPassant(_))));
    }

    #[test]
    fn reject_invalid_clocks() {
        // Invalid halfmove clock
        let result =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - abc 1");
        assert!(matches!(result, Err(FenError::InvalidHalfmoveClock(_))));

        // Fullmove number 0
        let result = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 0");
        assert!(matches!(result, Err(FenError::InvalidFullmoveNumber(_))));

        // Fullmove number -1 (will fail u16 parse)
        let result =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 -1");
        assert!(matches!(result, Err(FenError::InvalidFullmoveNumber(_))));
    }
}
