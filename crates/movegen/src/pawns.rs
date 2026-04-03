use chess_board::Position;
use chess_types::{Bitboard, Color, Move, MoveFlag, Piece, PieceKind, Square};

const NOT_A_FILE: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
const NOT_H_FILE: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);
const RANK_3: Bitboard = Bitboard::new(0x0000_0000_00FF_0000);
const RANK_6: Bitboard = Bitboard::new(0x0000_FF00_0000_0000);
const RANK_8: Bitboard = Bitboard::new(0xFF00_0000_0000_0000);
const RANK_1: Bitboard = Bitboard::new(0x0000_0000_0000_00FF);

pub fn generate_pawn_moves(pos: &Position, moves: &mut Vec<Move>) {
    let us = pos.side_to_move();
    let them = us.opposite();
    let pawns = pos.piece_bitboard(Piece::new(us, PieceKind::Pawn));
    let empty = !pos.occupied();
    let enemies = pos.occupied_by(them);

    match us {
        Color::White => generate_white(pawns, empty, enemies, pos.en_passant(), moves),
        Color::Black => generate_black(pawns, empty, enemies, pos.en_passant(), moves),
    }
}

fn generate_white(
    pawns: Bitboard,
    empty: Bitboard,
    enemies: Bitboard,
    ep: Option<Square>,
    moves: &mut Vec<Move>,
) {
    let promo_rank = RANK_8;

    // Single push
    let single = (pawns << 8) & empty;
    let single_no_promo = single & !promo_rank;
    let single_promo = single & promo_rank;

    for to_idx in single_no_promo {
        let from = Square::new((to_idx as u8) - 8).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::QUIET));
    }

    for to_idx in single_promo {
        let from = Square::new((to_idx as u8) - 8).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION));
    }

    // Double push
    let double = (((pawns << 8) & empty & RANK_3) << 8) & empty;
    for to_idx in double {
        let from = Square::new((to_idx as u8) - 16).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::DOUBLE_PAWN_PUSH));
    }

    // Captures: up-left (<< 7, NOT_A_FILE applied to targets to prevent wrap from H-file pawns)
    let cap_left = ((pawns & NOT_A_FILE) << 7) & enemies;
    let cap_left_no_promo = cap_left & !promo_rank;
    let cap_left_promo = cap_left & promo_rank;

    for to_idx in cap_left_no_promo {
        let from = Square::new((to_idx as u8) - 7).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::CAPTURE));
    }

    for to_idx in cap_left_promo {
        let from = Square::new((to_idx as u8) - 7).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION_CAPTURE));
    }

    // Captures: up-right (<< 9, NOT_H_FILE applied to prevent wrap from A-file pawns... wait)
    // Actually: white up-right is << 9. A pawn on h-file shifted << 9 wraps to a-file of next rank+2.
    // We need NOT_H_FILE on the pawns before shifting.
    let cap_right = ((pawns & NOT_H_FILE) << 9) & enemies;
    let cap_right_no_promo = cap_right & !promo_rank;
    let cap_right_promo = cap_right & promo_rank;

    for to_idx in cap_right_no_promo {
        let from = Square::new((to_idx as u8) - 9).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::CAPTURE));
    }

    for to_idx in cap_right_promo {
        let from = Square::new((to_idx as u8) - 9).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION_CAPTURE));
    }

    // En passant
    if let Some(ep_sq) = ep {
        let ep_bb = Bitboard::new(1u64 << ep_sq.index());

        // Up-left direction
        let ep_left = ((pawns & NOT_A_FILE) << 7) & ep_bb;
        if !ep_left.is_empty() {
            let from = Square::new(ep_sq.index() - 7).expect("valid square");
            moves.push(Move::new(from, ep_sq, MoveFlag::EN_PASSANT));
        }

        // Up-right direction
        let ep_right = ((pawns & NOT_H_FILE) << 9) & ep_bb;
        if !ep_right.is_empty() {
            let from = Square::new(ep_sq.index() - 9).expect("valid square");
            moves.push(Move::new(from, ep_sq, MoveFlag::EN_PASSANT));
        }
    }
}

fn generate_black(
    pawns: Bitboard,
    empty: Bitboard,
    enemies: Bitboard,
    ep: Option<Square>,
    moves: &mut Vec<Move>,
) {
    let promo_rank = RANK_1;

    // Single push
    let single = (pawns >> 8) & empty;
    let single_no_promo = single & !promo_rank;
    let single_promo = single & promo_rank;

    for to_idx in single_no_promo {
        let from = Square::new((to_idx as u8) + 8).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::QUIET));
    }

    for to_idx in single_promo {
        let from = Square::new((to_idx as u8) + 8).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION));
    }

    // Double push
    let double = (((pawns >> 8) & empty & RANK_6) >> 8) & empty;
    for to_idx in double {
        let from = Square::new((to_idx as u8) + 16).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::DOUBLE_PAWN_PUSH));
    }

    // Captures: down-right (>> 7, NOT_H_FILE on pawns to prevent wrap)
    let cap_right = ((pawns & NOT_H_FILE) >> 7) & enemies;
    let cap_right_no_promo = cap_right & !promo_rank;
    let cap_right_promo = cap_right & promo_rank;

    for to_idx in cap_right_no_promo {
        let from = Square::new((to_idx as u8) + 7).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::CAPTURE));
    }

    for to_idx in cap_right_promo {
        let from = Square::new((to_idx as u8) + 7).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION_CAPTURE));
    }

    // Captures: down-left (>> 9, NOT_A_FILE on pawns to prevent wrap)
    let cap_left = ((pawns & NOT_A_FILE) >> 9) & enemies;
    let cap_left_no_promo = cap_left & !promo_rank;
    let cap_left_promo = cap_left & promo_rank;

    for to_idx in cap_left_no_promo {
        let from = Square::new((to_idx as u8) + 9).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::CAPTURE));
    }

    for to_idx in cap_left_promo {
        let from = Square::new((to_idx as u8) + 9).expect("valid square");
        let to = Square::new(to_idx as u8).expect("valid square");
        moves.push(Move::new(from, to, MoveFlag::KNIGHT_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::BISHOP_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::ROOK_PROMOTION_CAPTURE));
        moves.push(Move::new(from, to, MoveFlag::QUEEN_PROMOTION_CAPTURE));
    }

    // En passant
    if let Some(ep_sq) = ep {
        let ep_bb = Bitboard::new(1u64 << ep_sq.index());

        // Down-right direction
        let ep_right = ((pawns & NOT_H_FILE) >> 7) & ep_bb;
        if !ep_right.is_empty() {
            let from = Square::new(ep_sq.index() + 7).expect("valid square");
            moves.push(Move::new(from, ep_sq, MoveFlag::EN_PASSANT));
        }

        // Down-left direction
        let ep_left = ((pawns & NOT_A_FILE) >> 9) & ep_bb;
        if !ep_left.is_empty() {
            let from = Square::new(ep_sq.index() + 9).expect("valid square");
            moves.push(Move::new(from, ep_sq, MoveFlag::EN_PASSANT));
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
    fn test_white_single_push() {
        let pos = Position::startpos();
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        let single_pushes: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::QUIET)
            .collect();
        assert_eq!(single_pushes.len(), 8);
        for file in 0..8u8 {
            let from = Square::new(8 + file).expect("valid");
            let to = Square::new(16 + file).expect("valid");
            assert!(has_move(&moves, from, to, MoveFlag::QUIET));
        }
    }

    #[test]
    fn test_white_single_push_blocked() {
        // White pawn on e2, black knight on e3
        let pos = Position::from_fen("4k3/8/8/8/8/4n3/4P3/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(!has_move(&moves, Square::E2, Square::E3, MoveFlag::QUIET));
    }

    #[test]
    fn test_black_single_push() {
        // Black to move, pawns on rank 7
        let pos = Position::from_fen("4k3/pppppppp/8/8/8/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        let single_pushes: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::QUIET)
            .collect();
        assert_eq!(single_pushes.len(), 8);
        for file in 0..8u8 {
            let from = Square::new(48 + file).expect("valid");
            let to = Square::new(40 + file).expect("valid");
            assert!(has_move(&moves, from, to, MoveFlag::QUIET));
        }
    }

    #[test]
    fn test_white_double_push() {
        let pos = Position::startpos();
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        let double_pushes: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::DOUBLE_PAWN_PUSH)
            .collect();
        assert_eq!(double_pushes.len(), 8);
        for file in 0..8u8 {
            let from = Square::new(8 + file).expect("valid");
            let to = Square::new(24 + file).expect("valid");
            assert!(has_move(&moves, from, to, MoveFlag::DOUBLE_PAWN_PUSH));
        }
    }

    #[test]
    fn test_white_double_push_blocked_intermediate() {
        // White pawn on e2, piece on e3
        let pos = Position::from_fen("4k3/8/8/8/8/4n3/4P3/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(!has_move(
            &moves,
            Square::E2,
            Square::E4,
            MoveFlag::DOUBLE_PAWN_PUSH
        ));
    }

    #[test]
    fn test_white_double_push_blocked_target() {
        // White pawn on e2, e3 empty, piece on e4
        let pos = Position::from_fen("4k3/8/8/8/4n3/8/4P3/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(!has_move(
            &moves,
            Square::E2,
            Square::E4,
            MoveFlag::DOUBLE_PAWN_PUSH
        ));
    }

    #[test]
    fn test_black_double_push() {
        // Black to move, pawns on rank 7
        let pos = Position::from_fen("4k3/pppppppp/8/8/8/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        let double_pushes: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::DOUBLE_PAWN_PUSH)
            .collect();
        assert_eq!(double_pushes.len(), 8);
        for file in 0..8u8 {
            let from = Square::new(48 + file).expect("valid");
            let to = Square::new(32 + file).expect("valid");
            assert!(has_move(&moves, from, to, MoveFlag::DOUBLE_PAWN_PUSH));
        }
    }

    #[test]
    fn test_white_captures() {
        // White pawn on d4, black pieces on c5 and e5
        let pos = Position::from_fen("4k3/8/8/2n1n3/3P4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(&moves, Square::D4, Square::C5, MoveFlag::CAPTURE));
        assert!(has_move(&moves, Square::D4, Square::E5, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_black_captures() {
        // Black pawn on e5, white pieces on d4 and f4, black to move
        let pos = Position::from_fen("4k3/8/8/4p3/3N1N2/8/8/4K3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(&moves, Square::E5, Square::D4, MoveFlag::CAPTURE));
        assert!(has_move(&moves, Square::E5, Square::F4, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_no_capture_on_friendly() {
        // White pawn on d4, white piece on e5
        let pos = Position::from_fen("4k3/8/8/4N3/3P4/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(!has_move(&moves, Square::D4, Square::E5, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_capture_no_file_wrap() {
        // White pawn on a4 with black piece on h5 — should NOT wrap
        let pos = Position::from_fen("4k3/8/8/7n/P7/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(!has_move(&moves, Square::A4, Square::H5, MoveFlag::CAPTURE));
    }

    #[test]
    fn test_white_en_passant() {
        // White pawn on e5, ep square d6
        let pos = Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::E5,
            Square::D6,
            MoveFlag::EN_PASSANT
        ));
    }

    #[test]
    fn test_black_en_passant() {
        // Black pawn on d4, ep square e3
        let pos = Position::from_fen("4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::D4,
            Square::E3,
            MoveFlag::EN_PASSANT
        ));
    }

    #[test]
    fn test_no_en_passant_when_unavailable() {
        // No ep square
        let pos = Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.flags() == MoveFlag::EN_PASSANT)
            .collect();
        assert!(ep_moves.is_empty());
    }

    #[test]
    fn test_white_promotion() {
        // White pawn on e7, e8 empty
        let pos = Position::from_fen("7k/4P3/8/8/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::E7,
            Square::E8,
            MoveFlag::KNIGHT_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::E8,
            MoveFlag::BISHOP_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::E8,
            MoveFlag::ROOK_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::E8,
            MoveFlag::QUEEN_PROMOTION
        ));
        let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
        assert_eq!(promo_moves.len(), 4);
    }

    #[test]
    fn test_black_promotion() {
        // Black pawn on e2, e1 empty, black to move
        let pos = Position::from_fen("4k3/8/8/8/8/8/4p3/K7 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::E2,
            Square::E1,
            MoveFlag::KNIGHT_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::E1,
            MoveFlag::BISHOP_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::E1,
            MoveFlag::ROOK_PROMOTION
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::E1,
            MoveFlag::QUEEN_PROMOTION
        ));
        let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
        assert_eq!(promo_moves.len(), 4);
    }

    #[test]
    fn test_white_promotion_capture() {
        // White pawn on e7, black piece on d8
        let pos = Position::from_fen("3nk3/4P3/8/8/8/8/8/4K3 w - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::E7,
            Square::D8,
            MoveFlag::KNIGHT_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::D8,
            MoveFlag::BISHOP_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::D8,
            MoveFlag::ROOK_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E7,
            Square::D8,
            MoveFlag::QUEEN_PROMOTION_CAPTURE
        ));
    }

    #[test]
    fn test_black_promotion_capture() {
        // Black pawn on e2, white piece on d1, black to move
        let pos = Position::from_fen("4k3/8/8/8/8/8/4p3/3NK3 b - - 0 1").expect("valid fen");
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        assert!(has_move(
            &moves,
            Square::E2,
            Square::D1,
            MoveFlag::KNIGHT_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::D1,
            MoveFlag::BISHOP_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::D1,
            MoveFlag::ROOK_PROMOTION_CAPTURE
        ));
        assert!(has_move(
            &moves,
            Square::E2,
            Square::D1,
            MoveFlag::QUEEN_PROMOTION_CAPTURE
        ));
    }

    #[test]
    fn test_no_moves_for_opponent_pawns() {
        // Startpos (white to move) — should generate no moves for black pawns
        let pos = Position::startpos();
        let mut moves = Vec::new();
        generate_pawn_moves(&pos, &mut moves);

        for m in &moves {
            let from = m.from_sq();
            // All moves should originate from rank 2 (white pawns in startpos)
            assert!(
                from.index() >= 8 && from.index() < 16,
                "unexpected move from {:?}",
                from
            );
        }
    }
}
