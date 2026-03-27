pub mod bishops;
pub mod castling;
pub mod king;
pub mod knights;
pub mod magic;
pub mod pawns;
pub mod queens;
pub mod rooks;

pub use bishops::generate_bishop_moves;
pub use castling::generate_castling_moves;
pub use king::generate_king_moves;
pub use knights::generate_knight_moves;
pub use pawns::generate_pawn_moves;
pub use queens::generate_queen_moves;
pub use queens::generate_sliding_moves;
pub use rooks::generate_rook_moves;

use chess_board::Position;
use chess_types::{Move, Piece, PieceKind, Square};

pub fn generate_legal_moves(pos: &mut Position) -> Vec<Move> {
    let mut pseudo_legal = Vec::new();
    generate_pawn_moves(pos, &mut pseudo_legal);
    generate_knight_moves(pos, &mut pseudo_legal);
    generate_king_moves(pos, &mut pseudo_legal);
    generate_sliding_moves(pos, &mut pseudo_legal);
    generate_castling_moves(pos, &mut pseudo_legal);

    let us = pos.side_to_move();
    let king_piece = Piece::new(us, PieceKind::King);

    let mut legal = Vec::new();
    for mv in pseudo_legal {
        let undo = pos.make_move(mv);
        let king_sq = Square::new(
            pos.piece_bitboard(king_piece)
                .into_iter()
                .next()
                .expect("king must exist") as u8,
        )
        .expect("valid square");
        let in_check = pos.is_square_attacked(king_sq, pos.side_to_move());
        pos.unmake_move(mv, undo);
        if !in_check {
            legal.push(mv);
        }
    }

    legal
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn test_startpos_legal_moves() {
        let mut pos = Position::startpos();
        let moves = generate_legal_moves(&mut pos);
        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn test_position_unchanged_after_generate() {
        let mut pos = Position::startpos();
        let fen_before = pos.to_fen();
        let hash_before = pos.hash();
        let _ = generate_legal_moves(&mut pos);
        assert_eq!(pos.to_fen(), fen_before);
        assert_eq!(pos.hash(), hash_before);
    }

    #[test]
    fn test_pinned_piece_restriction() {
        // White bishop on e2 pinned by black rook on e7 to white king on e1.
        // Bishop can only move diagonally, but the pin is along the e-file,
        // so the bishop has zero legal moves.
        let mut pos = Position::from_fen("4k3/4r3/8/8/8/8/4B3/4K3 w - - 0 1").expect("valid fen");
        let moves = generate_legal_moves(&mut pos);
        let bishop_moves: Vec<_> = moves
            .iter()
            .filter(|m| {
                m.from_sq() == Square::from_file_rank(chess_types::File::E, chess_types::Rank::R2)
            })
            .collect();
        assert_eq!(bishop_moves.len(), 0);
    }

    #[test]
    fn test_en_passant_discovered_check() {
        // Black king on a4, white pawn on d4, black pawn on e4, white rook on h4.
        // EP square d3. Capturing exd3 en passant would remove both pawns from rank 4,
        // exposing the black king to the white rook. This EP must be filtered out.
        let mut pos = Position::from_fen("8/8/8/8/k2Pp2R/8/8/4K3 b - d3 0 1").expect("valid fen");
        let moves = generate_legal_moves(&mut pos);
        let ep_d3 = Square::from_file_rank(chess_types::File::D, chess_types::Rank::R3);
        let ep_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.is_en_passant() && m.to_sq() == ep_d3)
            .collect();
        assert_eq!(ep_moves.len(), 0);
    }

    #[test]
    fn test_discovered_check_filtered() {
        // White king on e1, black rook on e8, white knight on e4.
        // The knight is pinned along the e-file by the rook.
        // A knight can never stay on the same file, so all knight moves are illegal.
        let mut pos = Position::from_fen("4r3/8/8/8/4N3/8/8/4K2k w - - 0 1").expect("valid fen");
        let moves = generate_legal_moves(&mut pos);
        let knight_sq = Square::from_file_rank(chess_types::File::E, chess_types::Rank::R4);
        let knight_moves: Vec<_> = moves.iter().filter(|m| m.from_sq() == knight_sq).collect();
        assert_eq!(knight_moves.len(), 0);
    }
}
