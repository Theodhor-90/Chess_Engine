use chess_board::Position;
use chess_types::{Color, Piece, PieceKind};

pub const MAX_PHASE: i32 = 24;

const KNIGHT_PHASE: i32 = 1;
const BISHOP_PHASE: i32 = 1;
const ROOK_PHASE: i32 = 2;
const QUEEN_PHASE: i32 = 4;

pub fn compute_phase(pos: &Position) -> i32 {
    let phase_weights = [
        (PieceKind::Knight, KNIGHT_PHASE),
        (PieceKind::Bishop, BISHOP_PHASE),
        (PieceKind::Rook, ROOK_PHASE),
        (PieceKind::Queen, QUEEN_PHASE),
    ];

    let mut phase = 0i32;
    for (kind, weight) in phase_weights {
        let count = pos
            .piece_bitboard(Piece::new(Color::White, kind))
            .pop_count()
            + pos
                .piece_bitboard(Piece::new(Color::Black, kind))
                .pop_count();
        phase += count as i32 * weight;
    }

    phase.clamp(0, MAX_PHASE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_phase_is_max() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(compute_phase(&pos), MAX_PHASE);
    }

    #[test]
    fn king_and_pawns_phase_is_zero() {
        let pos = Position::from_fen("4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1").unwrap();
        assert_eq!(compute_phase(&pos), 0);
    }

    #[test]
    fn missing_one_queen_intermediate() {
        let pos =
            Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(compute_phase(&pos), 20);
    }

    #[test]
    fn promoted_pieces_clamped() {
        let pos = Position::from_fen("QQQQk3/8/8/8/8/8/8/QQQQK3 w - - 0 1").unwrap();
        assert_eq!(compute_phase(&pos), MAX_PHASE);
    }
}
