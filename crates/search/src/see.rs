use chess_board::Position;
use chess_types::{Bitboard, Color, Move, Piece, PieceKind, Square};

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

const SEE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

fn pawn_attacks(sq: Square, side: Color) -> Bitboard {
    let bb = Bitboard::new(1u64 << sq.index());
    match side {
        Color::White => ((bb & NOT_A_FILE) << 7) | ((bb & NOT_H_FILE) << 9),
        Color::Black => ((bb & NOT_H_FILE) >> 7) | ((bb & NOT_A_FILE) >> 9),
    }
}

fn attackers_to(pos: &Position, sq: Square, occupied: Bitboard) -> Bitboard {
    let knights = chess_movegen::knights::KNIGHT_ATTACKS[sq.index() as usize]
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Knight))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Knight)));

    let kings = chess_movegen::king::KING_ATTACKS[sq.index() as usize]
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::King))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::King)));

    let bishop_rays = chess_movegen::magic::bishop_attacks(sq, occupied);
    let rook_rays = chess_movegen::magic::rook_attacks(sq, occupied);

    let bishops_queens = bishop_rays
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop))
            | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));

    let rooks_queens = rook_rays
        & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Rook))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Rook))
            | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
            | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));

    let white_pawns = pawn_attacks(sq, Color::Black)
        & pos.piece_bitboard(Piece::new(Color::White, PieceKind::Pawn));
    let black_pawns = pawn_attacks(sq, Color::White)
        & pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Pawn));

    knights | kings | bishops_queens | rooks_queens | white_pawns | black_pawns
}

fn least_valuable_attacker(
    pos: &Position,
    attackers: Bitboard,
    side: Color,
) -> Option<(Square, PieceKind)> {
    for &kind in &[
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ] {
        let piece_bb = pos.piece_bitboard(Piece::new(side, kind)) & attackers;
        if !piece_bb.is_empty() {
            let sq_idx = piece_bb.lsb().expect("non-empty bitboard has lsb");
            let sq = Square::new(sq_idx as u8).expect("valid square index");
            return Some((sq, kind));
        }
    }
    None
}

pub fn see(pos: &Position, mv: Move) -> i32 {
    let from = mv.from_sq();
    let to = mv.to_sq();

    let victim_value = if mv.is_en_passant() {
        SEE_VALUES[PieceKind::Pawn as usize]
    } else {
        match pos.piece_on(to) {
            Some(p) => SEE_VALUES[p.kind as usize],
            None => 0,
        }
    };

    let attacker_piece = pos.piece_on(from).expect("attacker must exist on from_sq");
    let mut attacker_value = SEE_VALUES[attacker_piece.kind as usize];

    if mv.is_promotion() {
        if let Some(promo_kind) = mv.promotion_piece() {
            attacker_value = SEE_VALUES[promo_kind as usize];
        }
    }

    let mut gain = [0i32; 32];
    let mut depth = 0usize;

    gain[0] = victim_value;
    if mv.is_promotion() {
        if let Some(promo_kind) = mv.promotion_piece() {
            gain[0] += SEE_VALUES[promo_kind as usize] - SEE_VALUES[PieceKind::Pawn as usize];
        }
    }

    let mut occupied = pos.occupied();
    occupied ^= Bitboard::new(1u64 << from.index());

    if mv.is_en_passant() {
        let ep_victim_sq = match attacker_piece.color {
            Color::White => Square::new(to.index() - 8).expect("valid ep square"),
            Color::Black => Square::new(to.index() + 8).expect("valid ep square"),
        };
        occupied ^= Bitboard::new(1u64 << ep_victim_sq.index());
    }

    let mut attackers = attackers_to(pos, to, occupied) & occupied;

    let mut side = attacker_piece.color.opposite();

    loop {
        let side_attackers = attackers & pos.occupied_by(side);
        let lva = least_valuable_attacker(pos, side_attackers, side);
        let (attacker_sq, attacker_kind) = match lva {
            Some(pair) => pair,
            None => break,
        };

        depth += 1;
        if depth >= 32 {
            break;
        }

        gain[depth] = attacker_value - gain[depth - 1];

        attacker_value = SEE_VALUES[attacker_kind as usize];

        occupied ^= Bitboard::new(1u64 << attacker_sq.index());

        if matches!(
            attacker_kind,
            PieceKind::Pawn | PieceKind::Bishop | PieceKind::Queen
        ) {
            let diag = chess_movegen::magic::bishop_attacks(to, occupied);
            let diag_pieces = diag
                & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Bishop))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Bishop))
                    | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));
            attackers |= diag_pieces;
        }
        if matches!(attacker_kind, PieceKind::Rook | PieceKind::Queen) {
            let straight = chess_movegen::magic::rook_attacks(to, occupied);
            let straight_pieces = straight
                & (pos.piece_bitboard(Piece::new(Color::White, PieceKind::Rook))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Rook))
                    | pos.piece_bitboard(Piece::new(Color::White, PieceKind::Queen))
                    | pos.piece_bitboard(Piece::new(Color::Black, PieceKind::Queen)));
            attackers |= straight_pieces;
        }

        attackers &= occupied;

        side = side.opposite();

        if attacker_kind == PieceKind::King && !(attackers & pos.occupied_by(side)).is_empty() {
            depth -= 1;
            break;
        }
    }

    while depth > 0 {
        depth -= 1;
        gain[depth] = -i32::max(-gain[depth], gain[depth + 1]);
    }

    gain[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;
    use chess_movegen::generate_legal_moves;

    fn find_capture(pos: &mut Position, from: Square, to: Square) -> Move {
        let moves = generate_legal_moves(pos);
        moves
            .into_iter()
            .find(|m| m.from_sq() == from && m.to_sq() == to && m.is_capture())
            .expect("expected capture move must exist")
    }

    #[test]
    fn see_pxn_winning() {
        let mut pos = Position::from_fen("4k3/8/4n3/3P4/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 320);
    }

    #[test]
    fn see_nxb_defended_roughly_equal() {
        let mut pos = Position::from_fen("4k3/5p2/4b3/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::D4, Square::E6);
        assert_eq!(see(&pos, mv), 10);
    }

    #[test]
    fn see_bxr_winning() {
        let mut pos = Position::from_fen("4k3/8/8/3r4/8/8/6B1/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::G2, Square::D5);
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_rxq_winning() {
        let mut pos = Position::from_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::D1, Square::D5);
        assert_eq!(see(&pos, mv), 900);
    }

    #[test]
    fn see_pxn_with_recapture_by_knight() {
        let mut pos = Position::from_fen("4k3/2n5/4n3/3P4/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::D5, Square::E6);
        assert_eq!(see(&pos, mv), 220);
    }

    #[test]
    fn see_undefended_piece_capture() {
        let mut pos = Position::from_fen("4k3/8/4r3/8/3N4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::D4, Square::E6);
        assert_eq!(see(&pos, mv), 500);
    }

    #[test]
    fn see_multi_piece_exchange_losing() {
        let mut pos =
            Position::from_fen("q3k3/5b2/8/3p4/8/4N3/8/3RK3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(
            see(&pos, mv) < 0,
            "multi-piece exchange should be losing for initiator"
        );
    }

    #[test]
    fn see_capture_into_defended_pawn_chain() {
        let mut pos = Position::from_fen("4k3/8/4p3/3p4/8/4N3/8/4K3 w - - 0 1").expect("valid fen");
        let mv = find_capture(&mut pos, Square::E3, Square::D5);
        assert!(
            see(&pos, mv) < 0,
            "capturing into defended pawn chain should lose material"
        );
    }
}
