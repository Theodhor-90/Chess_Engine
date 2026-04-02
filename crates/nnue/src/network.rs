use chess_types::Color;

use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE};

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
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores all network weights and biases.
///
/// The network has four layers: input → L1 → L2 → output.
/// - Input → L1: `i16` weights and biases (accumulator layer).
/// - L1 → L2: `i8` weights, `i32` biases.
/// - L2 → output: `i8` weights, `i32` bias.
///
/// `input_weights` uses `Box<[i16]>` instead of a fixed-size array because the
/// 10,485,760-element array (~20 MB) would overflow the stack. `Box<[i16]>` guarantees
/// heap allocation without any stack intermediary.
pub struct Network {
    /// Input → L1 weights. Stored as a flat array: `HALFKP_FEATURES × L1_SIZE` elements.
    /// Column-major: weights for feature `f` start at index `f * L1_SIZE`.
    pub input_weights: Box<[i16]>,
    /// L1 biases (L1_SIZE elements).
    pub input_bias: Box<[i16; L1_SIZE]>,
    /// L1 → L2 weights. `L2_SIZE × (2 * L1_SIZE)` elements.
    /// The input to this layer is the concatenation of white and black perspectives.
    pub hidden1_weights: Box<[i8]>,
    /// L2 biases (L2_SIZE elements).
    pub hidden1_bias: Box<[i32; L2_SIZE]>,
    /// L2 → output weights (`OUTPUT_SIZE × L2_SIZE` elements, i.e., `L2_SIZE`).
    pub hidden2_weights: Box<[i8; L2_SIZE]>,
    /// Output bias (`OUTPUT_SIZE` elements, i.e., 1).
    pub hidden2_bias: i32,
}

impl Network {
    /// Allocates a network with all weights and biases zeroed (for testing).
    pub fn new_zeroed() -> Self {
        Self {
            input_weights: vec![0i16; HALFKP_FEATURES * L1_SIZE].into_boxed_slice(),
            input_bias: Box::new([0i16; L1_SIZE]),
            hidden1_weights: vec![0i8; L2_SIZE * 2 * L1_SIZE].into_boxed_slice(),
            hidden1_bias: Box::new([0i32; L2_SIZE]),
            hidden2_weights: Box::new([0i8; L2_SIZE]),
            hidden2_bias: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE};

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
        // Set some non-trivial weights for feature index 42.
        let feature_idx = 42;
        for i in 0..L1_SIZE {
            net.input_weights[feature_idx * L1_SIZE + i] = (i as i16) * 7 - 50;
        }

        let mut acc = Accumulator::new();
        acc.init_from_bias(&net.input_bias);
        let snapshot_white = acc.white;
        let snapshot_black = acc.black;

        // Add then remove the same feature for white perspective.
        acc.add_feature(Color::White, feature_idx, &net.input_weights);
        // Verify it actually changed.
        assert_ne!(acc.white, snapshot_white);
        acc.remove_feature(Color::White, feature_idx, &net.input_weights);
        assert_eq!(acc.white, snapshot_white);

        // Same for black perspective.
        acc.add_feature(Color::Black, feature_idx, &net.input_weights);
        assert_ne!(acc.black, snapshot_black);
        acc.remove_feature(Color::Black, feature_idx, &net.input_weights);
        assert_eq!(acc.black, snapshot_black);
    }

    #[test]
    fn network_new_zeroed() {
        let net = Network::new_zeroed();
        assert_eq!(net.input_weights.len(), HALFKP_FEATURES * L1_SIZE);
        assert!(net.input_weights.iter().all(|&w| w == 0));
        assert!(net.input_bias.iter().all(|&b| b == 0));
        assert_eq!(net.hidden1_weights.len(), L2_SIZE * 2 * L1_SIZE);
        assert!(net.hidden1_weights.iter().all(|&w| w == 0));
        assert!(net.hidden1_bias.iter().all(|&b| b == 0));
        assert!(net.hidden2_weights.iter().all(|&w| w == 0));
        assert_eq!(net.hidden2_bias, 0);
    }
}
