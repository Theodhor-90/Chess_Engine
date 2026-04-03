use chess_board::Position;
use chess_types::{Move, MoveFlag, Piece, PieceKind, Square};

pub fn generate_rook_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let rooks = pos.piece_bitboard(Piece::new(us, PieceKind::Rook));
    let friendly = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.opposite());
    let occupancy = pos.occupied();

    for from_idx in rooks {
        let from = Square::new(from_idx as u8).expect("valid square");
        let attacks = super::magic::rook_attacks(from, occupancy);
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
    fn test_rook_moves_empty_board() {
        let pos = Position::from_fen("4k3/8/8/8/3R4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_rook_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 14);
        assert!(moves.iter().all(|m| m.flags() == MoveFlag::QUIET));
    }

    #[test]
    fn test_rook_excludes_friendly() {
        let pos = Position::from_fen("4k3/8/8/8/3RN3/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_rook_moves(&pos, &mut moves);
        assert!(!has_move(&moves, Square::D4, Square::E4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::E4, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::F4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::G4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::H4, MoveFlag::QUIET));
    }

    #[test]
    fn test_rook_captures() {
        let pos = Position::from_fen("4k3/8/8/8/3Rn3/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_rook_moves(&pos, &mut moves);
        assert!(has_move(&moves, Square::D4, Square::E4, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::F4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::F4, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::G4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::G4, MoveFlag::CAPTURE));
        assert!(!has_move(&moves, Square::D4, Square::H4, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::H4, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_black_rook_moves() {
        let pos = Position::from_fen("4k3/8/8/8/3r4/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_rook_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 14);
    }
}
