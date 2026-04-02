use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE};

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
