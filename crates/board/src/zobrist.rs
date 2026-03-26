use chess_types::{Bitboard, Color, File, Piece, PieceKind, Square};

use crate::position::{CastlingRights, Position};

const fn xorshift64(mut state: u64) -> u64 {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

const fn generate_keys() -> ([u64; 768], u64, [u64; 16], [u64; 8]) {
    let mut state: u64 = 0x29A4_D52E_9652_73AB;
    let mut piece_square = [0u64; 768];
    let mut i = 0;
    while i < 768 {
        state = xorshift64(state);
        piece_square[i] = state;
        i += 1;
    }
    state = xorshift64(state);
    let side_to_move = state;

    let mut castling = [0u64; 16];
    i = 0;
    while i < 16 {
        state = xorshift64(state);
        castling[i] = state;
        i += 1;
    }

    let mut en_passant = [0u64; 8];
    i = 0;
    while i < 8 {
        state = xorshift64(state);
        en_passant[i] = state;
        i += 1;
    }

    (piece_square, side_to_move, castling, en_passant)
}

const GENERATED: ([u64; 768], u64, [u64; 16], [u64; 8]) = generate_keys();
const PIECE_SQUARE_KEYS: [u64; 768] = GENERATED.0;
const SIDE_TO_MOVE_KEY: u64 = GENERATED.1;
const CASTLING_KEYS: [u64; 16] = GENERATED.2;
const EN_PASSANT_KEYS: [u64; 8] = GENERATED.3;

#[inline]
pub fn piece_square_key(piece: Piece, square: Square) -> u64 {
    PIECE_SQUARE_KEYS[piece.index() * 64 + square.index() as usize]
}

#[inline]
pub fn side_to_move_key() -> u64 {
    SIDE_TO_MOVE_KEY
}

#[inline]
pub fn castling_key(rights: CastlingRights) -> u64 {
    CASTLING_KEYS[rights.inner() as usize]
}

#[inline]
pub fn en_passant_key(file: File) -> u64 {
    EN_PASSANT_KEYS[file as usize]
}

pub fn compute_hash(pos: &Position) -> u64 {
    let mut hash = 0u64;

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
            let mut bb = pos.piece_bitboard(piece);
            while !bb.is_empty() {
                let sq_idx = bb.inner().trailing_zeros() as u8;
                if let Some(sq) = Square::new(sq_idx) {
                    hash ^= piece_square_key(piece, sq);
                }
                bb = Bitboard::new(bb.inner() & (bb.inner() - 1));
            }
        }
    }

    if pos.side_to_move() == Color::Black {
        hash ^= SIDE_TO_MOVE_KEY;
    }

    hash ^= castling_key(pos.castling_rights());

    if let Some(ep_sq) = pos.en_passant() {
        let file_idx = ep_sq.index() % 8;
        hash ^= EN_PASSANT_KEYS[file_idx as usize];
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_hash_is_nonzero_and_deterministic() {
        let pos1 = Position::startpos();
        let pos2 = Position::startpos();
        assert_ne!(pos1.hash(), 0);
        assert_eq!(pos1.hash(), pos2.hash());
    }

    #[test]
    fn same_fen_produces_same_hash() {
        let fen = "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2";
        let pos1 = Position::from_fen(fen).unwrap();
        let pos2 = Position::from_fen(fen).unwrap();
        assert_eq!(pos1.hash(), pos2.hash());

        let startpos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let from_fen = Position::from_fen(startpos_fen).unwrap();
        let startpos = Position::startpos();
        assert_eq!(from_fen.hash(), startpos.hash());
    }

    #[test]
    fn different_positions_produce_different_hashes() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
            "r1bqkb1r/pppppppp/2n2n2/8/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
            "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 2 4",
            "8/8/4k3/8/8/4K3/4P3/8 w - - 0 1",
        ];
        let hashes: Vec<u64> = fens
            .iter()
            .map(|fen| Position::from_fen(fen).unwrap().hash())
            .collect();

        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "collision between FEN[{}] and FEN[{}]",
                    i, j
                );
            }
        }
    }

    #[test]
    fn incremental_update_matches_full_recomputation() {
        let pos_before =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let pos_after =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();

        let white_pawn = Piece::new(Color::White, PieceKind::Pawn);
        let mut hash = pos_before.hash();

        // XOR out white pawn on e2
        hash ^= piece_square_key(white_pawn, Square::E2);
        // XOR in white pawn on e4
        hash ^= piece_square_key(white_pawn, Square::E4);
        // XOR side-to-move
        hash ^= side_to_move_key();
        // XOR in en passant file e
        hash ^= en_passant_key(File::E);

        assert_eq!(hash, pos_after.hash());
    }

    #[test]
    fn from_fen_and_startpos_same_hash() {
        let startpos = Position::startpos();
        let from_fen =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(startpos.hash(), from_fen.hash());
    }
}
