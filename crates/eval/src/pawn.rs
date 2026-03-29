use chess_board::zobrist::piece_square_key;
use chess_board::Position;
use chess_types::{Color, Piece, PieceKind, Square};

const DEFAULT_SIZE: usize = 16_384;

#[derive(Clone, Copy, Default)]
pub struct PawnHashEntry {
    key: u64,
    mg: i32,
    eg: i32,
}

pub struct PawnHashTable {
    entries: Vec<PawnHashEntry>,
    mask: usize,
}

impl Default for PawnHashTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PawnHashTable {
    pub fn new() -> Self {
        Self {
            entries: vec![PawnHashEntry::default(); DEFAULT_SIZE],
            mask: DEFAULT_SIZE - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<(i32, i32)> {
        let index = (key as usize) & self.mask;
        let entry = self.entries[index];
        if entry.key == key {
            Some((entry.mg, entry.eg))
        } else {
            None
        }
    }

    pub fn store(&mut self, key: u64, mg: i32, eg: i32) {
        let index = (key as usize) & self.mask;
        self.entries[index] = PawnHashEntry { key, mg, eg };
    }
}

pub fn pawn_zobrist_hash(pos: &Position) -> u64 {
    let mut hash: u64 = 0;
    for color in [Color::White, Color::Black] {
        let bb = pos.piece_bitboard(Piece::new(color, PieceKind::Pawn));
        for sq_idx in bb {
            let sq = Square::new(sq_idx as u8).unwrap();
            hash ^= piece_square_key(Piece::new(color, PieceKind::Pawn), sq);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_miss_returns_none() {
        let table = PawnHashTable::new();
        assert_eq!(table.probe(0xDEAD_BEEF), None);
    }

    #[test]
    fn store_then_probe_returns_correct_scores() {
        let mut table = PawnHashTable::new();
        table.store(0x1234, 50, 30);
        assert_eq!(table.probe(0x1234), Some((50, 30)));
    }

    #[test]
    fn different_pawn_configs_produce_different_keys() {
        let startpos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let moved =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();
        let key1 = pawn_zobrist_hash(&startpos);
        let key2 = pawn_zobrist_hash(&moved);
        assert_ne!(key1, key2);
    }
}
