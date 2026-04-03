use chess_board::Position;
use chess_types::{Bitboard, Move, MoveFlag, Piece, PieceKind, Square};

const fn init_king_attacks() -> [Bitboard; 64] {
    let mut table = [Bitboard::new(0); 64];
    let offsets: [(i8, i8); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
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

const KING_ATTACKS: [Bitboard; 64] = init_king_attacks();

pub fn generate_king_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let them = us.opposite();
    let king_bb = pos.piece_bitboard(Piece::new(us, PieceKind::King));

    let king_sq_idx = match king_bb.lsb() {
        Some(idx) => idx,
        None => return,
    };

    let from = match Square::new(king_sq_idx as u8) {
        Some(sq) => sq,
        None => return,
    };

    let attacks = KING_ATTACKS[king_sq_idx as usize];
    let friendly = pos.occupied_by(us);
    let enemies = pos.occupied_by(them);
    let candidates = attacks & !friendly;

    for to_idx in candidates {
        let to = match Square::new(to_idx as u8) {
            Some(sq) => sq,
            None => continue,
        };

        if pos.is_square_attacked(to, them) {
            continue;
        }

        if !(Bitboard::new(1u64 << to_idx) & enemies).is_empty() {
            moves.push(Move::new(from, to, MoveFlag::CAPTURE));
        } else {
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
    fn test_king_attacks_corner_a1() {
        let attacks = KING_ATTACKS[Square::A1.index() as usize];
        assert_eq!(attacks.pop_count(), 3);
        assert!(!(attacks & Bitboard::new(1u64 << Square::B1.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::A2.index())).is_empty());
        assert!(!(attacks & Bitboard::new(1u64 << Square::B2.index())).is_empty());
    }

    #[test]
    fn test_king_attacks_edge_a4() {
        let attacks = KING_ATTACKS[Square::A4.index() as usize];
        assert_eq!(attacks.pop_count(), 5);
    }

    #[test]
    fn test_king_attacks_center_d4() {
        let attacks = KING_ATTACKS[Square::D4.index() as usize];
        assert_eq!(attacks.pop_count(), 8);
    }

    #[test]
    fn test_king_excludes_friendly() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/4N3/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_king_moves(&pos, &mut moves);
        assert!(!has_move(&moves, Square::E1, Square::E2, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::E1, Square::E2, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_king_excludes_attacked_squares() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/r3K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_king_moves(&pos, &mut moves);
        assert!(!has_move(&moves, Square::E1, Square::D1, MoveFlag::QUIET));
        assert!(!has_move(&moves, Square::E1, Square::D1, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_king_captures() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/4n3/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_king_moves(&pos, &mut moves);
        assert!(has_move(&moves, Square::E1, Square::E2, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_king_quiet_moves() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_king_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 5);
        assert!(moves.iter().all(|m| m.flags() == MoveFlag::QUIET));
    }

    #[test]
    fn test_black_king_moves() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_king_moves(&pos, &mut moves);
        assert_eq!(moves.len(), 5);
        assert!(has_move(&moves, Square::E8, Square::D8, MoveFlag::QUIET));
        assert!(has_move(&moves, Square::E8, Square::F8, MoveFlag::QUIET));
        assert!(has_move(&moves, Square::E8, Square::D7, MoveFlag::QUIET));
        assert!(has_move(&moves, Square::E8, Square::E7, MoveFlag::QUIET));
        assert!(has_move(&moves, Square::E8, Square::F7, MoveFlag::QUIET));
    }
}
