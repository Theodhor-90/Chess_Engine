use chess_board::Position;
use chess_types::{Move, MoveFlag, Piece, PieceKind, Square};

pub fn generate_bishop_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let bishops = pos.piece_bitboard(Piece::new(us, PieceKind::Bishop));
    let friendly = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.opposite());
    let occupancy = pos.occupied();

    for from_idx in bishops {
        let from = Square::new(from_idx as u8).expect("valid square");
        let attacks = super::magic::bishop_attacks(from, occupancy);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    fn has_move(moves: &[Move], from: Square, to: Square, flags: u16) -> bool {
        moves.contains(&Move::new(from, to, flags))
    }

    #[test]
    fn test_bishop_moves_empty_board() {
        let pos = Position::from_fen("4k3/8/8/8/3B4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_bishop_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 13);
        assert!(moves.iter().all(|m| m.flags() == MoveFlag::QUIET));
    }

    #[test]
    fn test_bishop_excludes_friendly() {
        let pos = Position::from_fen("4k3/8/8/2N5/3B4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_bishop_moves(&pos, &mut moves);
        assert!(!has_move(&moves, Square::D4, Square::C5, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::C5, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::B6, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::B6, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_bishop_captures() {
        let pos = Position::from_fen("4k3/8/8/2n5/3B4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_bishop_moves(&pos, &mut moves);
        assert!(has_move(&moves, Square::D4, Square::C5, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::B6, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::B6, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_black_bishop_moves() {
        let pos = Position::from_fen("4k3/8/8/8/3b4/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_bishop_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 13);
    }
}
