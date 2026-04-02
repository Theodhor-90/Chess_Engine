use chess_board::Position;
use chess_types::{Color, Piece, PieceKind, Square};

use crate::arch::L1_SIZE;
use crate::feature::feature_index;
use crate::network::Network;

/// Stores the incrementally updatable state for hidden layer 1.
///
/// Each perspective (white and black) has its own accumulator vector of `L1_SIZE`
/// elements. The accumulator is designed to support incremental updates: features
/// can be added or removed without full recomputation.
pub struct Accumulator {
    /// White perspective accumulator values (L1_SIZE elements).
    pub white: [i16; L1_SIZE],
    /// Black perspective accumulator values (L1_SIZE elements).
    pub black: [i16; L1_SIZE],
}

impl Accumulator {
    /// Creates a zeroed accumulator.
    pub fn new() -> Self {
        Self {
            white: [0; L1_SIZE],
            black: [0; L1_SIZE],
        }
    }

    /// Sets both perspectives to the bias values (used when refreshing from scratch).
    pub fn init_from_bias(&mut self, bias: &[i16; L1_SIZE]) {
        self.white = *bias;
        self.black = *bias;
    }

    /// Adds the weight column at `index` to the given perspective's accumulator.
    ///
    /// For each `i in 0..L1_SIZE`, adds `weights[index * L1_SIZE + i]` to the
    /// perspective's accumulator value at position `i`.
    pub fn add_feature(&mut self, perspective: Color, index: usize, weights: &[i16]) {
        let vals = match perspective {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };
        let offset = index * L1_SIZE;
        for i in 0..L1_SIZE {
            vals[i] += weights[offset + i];
        }
    }

    /// Subtracts the weight column at `index` from the given perspective's accumulator.
    ///
    /// For each `i in 0..L1_SIZE`, subtracts `weights[index * L1_SIZE + i]` from the
    /// perspective's accumulator value at position `i`.
    pub fn remove_feature(&mut self, perspective: Color, index: usize, weights: &[i16]) {
        let vals = match perspective {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };
        let offset = index * L1_SIZE;
        for i in 0..L1_SIZE {
            vals[i] -= weights[offset + i];
        }
    }

    /// Initializes the accumulator from a full position.
    ///
    /// Sets both perspectives to the L1 bias, then iterates all non-king pieces
    /// on the board and adds their corresponding HalfKP feature weight columns.
    pub fn refresh(&mut self, pos: &Position, network: &Network) {
        self.init_from_bias(&network.input_bias);

        let non_king_kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ];

        for perspective in [Color::White, Color::Black] {
            let king_piece = Piece::new(perspective, PieceKind::King);
            let king_sq = match pos.piece_bitboard(king_piece).lsb() {
                Some(idx) => Square::new(idx as u8).expect("valid king square"),
                None => continue,
            };

            for &piece_color in &[Color::White, Color::Black] {
                for &piece_kind in &non_king_kinds {
                    let piece = Piece::new(piece_color, piece_kind);
                    for sq_idx in pos.piece_bitboard(piece) {
                        let piece_sq = Square::new(sq_idx as u8).expect("valid square");
                        let idx = feature_index(king_sq, piece_sq, piece_kind, piece_color);
                        self.add_feature(perspective, idx, &network.input_weights);
                    }
                }
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::L1_SIZE;

    #[test]
    fn accumulator_init_from_bias() {
        let mut acc = Accumulator::new();
        let mut bias = [0i16; L1_SIZE];
        for (i, b) in bias.iter_mut().enumerate() {
            *b = (i as i16) * 3 - 100;
        }
        acc.init_from_bias(&bias);
        assert_eq!(acc.white, bias);
        assert_eq!(acc.black, bias);
    }

    #[test]
    fn accumulator_add_remove_roundtrip() {
        let mut net = Network::new_zeroed();
        let feature_idx = 42;
        for i in 0..L1_SIZE {
            net.input_weights[feature_idx * L1_SIZE + i] = (i as i16) * 7 - 50;
        }

        let mut acc = Accumulator::new();
        acc.init_from_bias(&net.input_bias);
        let snapshot_white = acc.white;
        let snapshot_black = acc.black;

        acc.add_feature(Color::White, feature_idx, &net.input_weights);
        assert_ne!(acc.white, snapshot_white);
        acc.remove_feature(Color::White, feature_idx, &net.input_weights);
        assert_eq!(acc.white, snapshot_white);

        acc.add_feature(Color::Black, feature_idx, &net.input_weights);
        assert_ne!(acc.black, snapshot_black);
        acc.remove_feature(Color::Black, feature_idx, &net.input_weights);
        assert_eq!(acc.black, snapshot_black);
    }

    #[test]
    fn accumulator_refresh_startpos() {
        let mut net = Network::new_zeroed();
        // Set deterministic non-zero input weights.
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 256) as i16 - 128;
        }
        let mut bias = [0i16; L1_SIZE];
        for (i, b) in bias.iter_mut().enumerate() {
            *b = (i as i16) % 50 - 25;
        }
        *net.input_bias = bias;

        let pos = Position::startpos();
        let mut acc = Accumulator::new();
        acc.refresh(&pos, &net);

        // Manually compute expected accumulator for both perspectives.
        let non_king_kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ];

        for perspective in [Color::White, Color::Black] {
            let mut expected = bias;
            let king_piece = Piece::new(perspective, PieceKind::King);
            let king_sq_idx = pos.piece_bitboard(king_piece).lsb().unwrap();
            let king_sq = Square::new(king_sq_idx as u8).unwrap();

            for &piece_color in &[Color::White, Color::Black] {
                for &piece_kind in &non_king_kinds {
                    let piece = Piece::new(piece_color, piece_kind);
                    for sq_idx in pos.piece_bitboard(piece) {
                        let piece_sq = Square::new(sq_idx as u8).unwrap();
                        let fi = feature_index(king_sq, piece_sq, piece_kind, piece_color);
                        let offset = fi * L1_SIZE;
                        for i in 0..L1_SIZE {
                            expected[i] += net.input_weights[offset + i];
                        }
                    }
                }
            }

            let actual = match perspective {
                Color::White => &acc.white,
                Color::Black => &acc.black,
            };
            assert_eq!(
                actual, &expected,
                "Accumulator mismatch for {:?} perspective",
                perspective
            );
        }

        // Verify that the accumulator was actually populated (not just bias).
        assert_ne!(acc.white, bias, "White accumulator should differ from bias");
        assert_ne!(acc.black, bias, "Black accumulator should differ from bias");
    }
}
