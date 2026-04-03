use chess_board::Position;
use chess_types::{Move, MoveFlag, Piece, PieceKind, Square};

pub fn generate_queen_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let queens = pos.piece_bitboard(Piece::new(us, PieceKind::Queen));
    let friendly = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.opposite());
    let occupancy = pos.occupied();

    for from_idx in queens {
        let from = Square::new(from_idx as u8).expect("valid square");
        let attacks = super::magic::bishop_attacks(from, occupancy)
            | super::magic::rook_attacks(from, occupancy);
        let targets = attacks & !friendly;
        let captures = targets & enemies;
        let quiets = targets & !enemies;

        for to_idx in captures {
            let to = Square::new(to_idx as u8).expect("valid square");
            moves.push(Move::new(from, to, MoveFlag::CAPTURE));
        }

        for to_idx in quiets {
            let to = Square::new(to_idx as u8).expect("valid square");
            moves.push(Move::new(from, to, MoveFlag::QUIET));
        }
    }
}

pub fn generate_sliding_moves(pos: &Position, moves: &mut Vec<Move>) {
    super::bishops::generate_bishop_moves(pos, moves);
    super::rooks::generate_rook_moves(pos, moves);
    generate_queen_moves(pos, moves);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn test_queen_moves_starting_position() {
        let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_queen_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_queen_moves_midgame() {
        let pos =
            Position::from_fen("r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 5 4")
                .expect("valid fen");
        let mut moves = Vec::new();
        generate_queen_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_queen_moves_blocked_by_pawns() {
        let pos = Position::from_fen("4k3/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1")
            .expect("valid fen");
        let mut moves = Vec::new();
        generate_queen_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_queen_moves_empty_board() {
        let pos = Position::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_queen_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 27);
        assert!(moves.iter().all(|m| m.flags() == MoveFlag::QUIET));
    }

    #[test]
    fn test_generate_sliding_moves_combined() {
        let pos = Position::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_sliding_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 27);
    }
}
