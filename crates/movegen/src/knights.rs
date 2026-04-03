use chess_board::Position;
use chess_types::{Bitboard, Move, MoveFlag, Piece, PieceKind, Square};

const fn init_knight_attacks() -> [Bitboard; 64] {
    let mut table = [Bitboard::new(0); 64];
    let offsets: [(i8, i8); 8] = [
        (-2, -1),
        (-2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (2, -1),
        (2, 1),
    ];
    let mut sq: usize = 0;
    while sq < 64 {
        let file = (sq % 8) as i8;
        let rank = (sq / 8) as i8;
        let mut bits: u64 = 0;
        let mut i = 0;
        while i < 8 {
            let tf = file + offsets[i].0;
            let tr = rank + offsets[i].1;
            if tf >= 0 && tf < 8 && tr >= 0 && tr < 8 {
                bits |= 1u64 << (tr as usize * 8 + tf as usize);
            }
            i += 1;
        }
        table[sq] = Bitboard::new(bits);
        sq += 1;
    }
    table
}

pub const KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attacks();

pub fn generate_knight_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let knights = pos.piece_bitboard(Piece::new(us, PieceKind::Knight));
    let friendly = pos.occupied_by(us);
    let enemies = pos.occupied_by(us.opposite());

    for from_idx in knights {
        let attacks = KNIGHT_ATTACKS[from_idx as usize];
        let targets = attacks & !friendly;
        let captures = targets & enemies;
        let quiets = targets & !enemies;
        let from = Square::new(from_idx as u8).expect("valid square");

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
    fn test_knight_attacks_corner_a1() {
        let attacks = KNIGHT_ATTACKS[Square::A1.index() as usize];
        assert_eq!(attacks.pop_count(), 2);
        assert!(!(attacks & Bitboard::new(1u64 << Square::B3.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::C2.index())).is_empty());
    }

    #[test]
    fn test_knight_attacks_edge_a4() {
        let attacks = KNIGHT_ATTACKS[Square::A4.index() as usize];
        assert_eq!(attacks.pop_count(), 4);
        assert!(!(attacks & Bitboard::new(1u64 << Square::B2.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::C3.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::C5.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::B6.index())).is_empty());
    }

    #[test]
    fn test_knight_attacks_center_d4() {
        let attacks = KNIGHT_ATTACKS[Square::D4.index() as usize];
        assert_eq!(attacks.pop_count(), 8);
    }

    #[test]
    fn test_knight_moves_center() {
        let pos = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_knight_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 8);
        assert!(moves.iter().all(|m| m.flags() == MoveFlag::QUIET));
    }

    #[test]
    fn test_knight_excludes_friendly() {
        let pos = Position::from_fen("4k3/8/2N5/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_knight_moves(&pos, &mut moves);
        assert!(!has_move(&moves, Square::D4, Square::C6, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::D4, Square::C6, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_knight_captures() {
        let pos = Position::from_fen("4k3/8/2n5/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_knight_moves(&pos, &mut moves);
        let d4_moves: Vec<_> = moves.iter().filter(|m| m.from_sq() == Square::D4).collect();
        assert!(has_move(&moves, Square::D4, Square::C6, MoveFlag::CAPTURE));
        let d4_quiets: Vec<_> = d4_moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::QUIET)
            .collect();
        assert_eq!(d4_quiets.len(), 7);
    }

    #[test]
    fn test_black_knight_moves() {
        let pos = Position::from_fen("4k3/8/8/8/3n4/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_knight_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 8);
    }
}
