use chess_types::Color;

use crate::accumulator::Accumulator;
use crate::arch::{L1_SIZE, L2_SIZE, QUANTIZATION_SHIFT_HIDDEN};
use crate::network::Network;
use crate::simd;

/// Performs the forward pass through the network, producing an evaluation score.
///
/// The accumulator must already be populated (via `refresh` or incremental updates).
/// Returns an `i32` score in centipawn-scale (positive = good for `side_to_move`).
pub fn forward(acc: &Accumulator, network: &Network, side_to_move: Color) -> i32 {
    // Step 1: Concatenate perspectives based on side to move.
    let (own, opp) = match side_to_move {
        Color::White => (&acc.white, &acc.black),
        Color::Black => (&acc.black, &acc.white),
    };

    // Step 2: Apply clipped-ReLU to the concatenated L1 input.
    let mut l1_input = [0i16; 2 * L1_SIZE];
    l1_input[..L1_SIZE].copy_from_slice(own);
    l1_input[L1_SIZE..].copy_from_slice(opp);

    let mut l1_output = [0i8; 2 * L1_SIZE];
    clipped_relu(&l1_input, &mut l1_output);

    // Step 3: L1 → L2 propagation (matrix-vector multiply).
    let mut l2_values = [0i32; L2_SIZE];
    let input_len = 2 * L1_SIZE;
    for (row, l2_val) in l2_values.iter_mut().enumerate() {
        let row_offset = row * input_len;
        let row_weights = &network.hidden1_weights[row_offset..row_offset + input_len];
        *l2_val = simd::dot_i8_i32(row_weights, &l1_output) + network.hidden1_bias[row];
    }

    // Step 4: Apply clipped-ReLU to L2 values (rescale first).
    let mut l2_rescaled = [0i16; L2_SIZE];
    for i in 0..L2_SIZE {
        l2_rescaled[i] = (l2_values[i] >> QUANTIZATION_SHIFT_HIDDEN) as i16;
    }
    let mut l2_output = [0i8; L2_SIZE];
    clipped_relu(&l2_rescaled, &mut l2_output);

    // Step 5: L2 → Output propagation (dot product).
    let output =
        simd::dot_i8_i32(network.hidden2_weights.as_ref(), &l2_output) + network.hidden2_bias;

    // Step 6: Return final score rescaled from quantized domain.
    output >> QUANTIZATION_SHIFT_HIDDEN
}

/// Applies clipped-ReLU: clamps each `i16` to `[0, CLIPPED_RELU_MAX]` and casts to `i8`.
fn clipped_relu(input: &[i16], output: &mut [i8]) {
    simd::clipped_relu_i16_to_i8(input, output);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::L1_SIZE;

    #[test]
    fn forward_zeroed_network() {
        let network = Network::new_zeroed();
        let acc = Accumulator::new();
        let result = forward(&acc, &network, Color::White);
        assert_eq!(result, 0);
    }

    #[test]
    fn forward_known_reference() {
        let mut network = Network::new_zeroed();

        // Set hidden1_weights: all 1s.
        for w in network.hidden1_weights.iter_mut() {
            *w = 1;
        }
        // Set hidden1_bias: all 0.
        // (already zeroed)

        // Set hidden2_weights: all 1s.
        for w in network.hidden2_weights.iter_mut() {
            *w = 1;
        }
        // hidden2_bias = 0 (already zeroed)

        // Set accumulator with known values.
        let mut acc = Accumulator::new();
        for i in 0..L1_SIZE {
            acc.white[i] = i as i16;
            acc.black[i] = (L1_SIZE - 1 - i) as i16;
        }

        let result = forward(&acc, &network, Color::White);

        // Hand-compute expected:
        // L1 input = [acc.white, acc.black] (512 i16 values).
        // After clipped-ReLU: values clamped to [0, 127]. white[i] = min(i, 127),
        //   black[i] = min(255-i, 127).
        // For white: i in 0..256 => min(i, 127) => 0,1,...,127,127,...,127
        //   Sum of clipped white = 0+1+...+127 + 127*128 = (127*128/2) + 127*128
        //     = 8128 + 16256 = 24384
        // For black: 255-i for i in 0..256 => 255,254,...,0 => clipped: 127,127,...127,127,...,0
        //   By symmetry, same sum = 24384
        // L1 output (i8) dot with hidden1_weights (all 1s) for each L2 row:
        //   Each row gets sum of all 512 clipped values = 24384 + 24384 = 48768.
        //   l2_values[row] = 48768 + 0 (bias) = 48768.
        // Rescaled: 48768 >> 6 = 762.
        // Clipped-ReLU on 762: min(762, 127) = 127, cast to i8.
        // L2 output: all 32 values = 127.
        // Dot product with hidden2_weights (all 1s): 127 * 32 = 4064.
        // Plus hidden2_bias (0): 4064.
        // Final: 4064 >> 6 = 63.
        assert_eq!(result, 63);
    }

    #[test]
    fn forward_perspective_swap() {
        let mut network = Network::new_zeroed();
        for w in network.hidden1_weights.iter_mut() {
            *w = 1;
        }
        for w in network.hidden2_weights.iter_mut() {
            *w = 1;
        }

        // Create an asymmetric accumulator.
        let mut acc = Accumulator::new();
        for i in 0..L1_SIZE {
            acc.white[i] = (i as i16) % 100;
            acc.black[i] = ((i as i16) % 100) + 10;
        }

        let white_result = forward(&acc, &network, Color::White);
        let black_result = forward(&acc, &network, Color::Black);

        // With all-1 weights in hidden layers, perspective swap means the L1 input
        // order changes: [white, black] vs [black, white]. But since hidden1_weights
        // are all 1s, the sum across the full 512-element input is the same regardless
        // of order. So both results should be equal.
        assert_eq!(white_result, black_result);

        // Now test with non-uniform hidden1_weights to show perspective matters.
        // Use small accumulator values with different sums for white vs black.
        let mut acc2 = Accumulator::new();
        for i in 0..L1_SIZE {
            acc2.white[i] = 1; // sum = 256
            acc2.black[i] = 2; // sum = 512
        }

        let mut network2 = Network::new_zeroed();
        // Asymmetric: only own-perspective columns (first L1_SIZE) have weight 1.
        // Opponent columns (second L1_SIZE) have weight 0.
        // This means the L2 value depends on which half is "own" perspective.
        for (i, w) in network2.hidden1_weights.iter_mut().enumerate() {
            let col = i % (2 * L1_SIZE);
            if col < L1_SIZE {
                *w = 1;
            }
        }
        for w in network2.hidden2_weights.iter_mut() {
            *w = 1;
        }

        let white_result2 = forward(&acc2, &network2, Color::White);
        let black_result2 = forward(&acc2, &network2, Color::Black);

        // White-to-move: own=white (sum 256), opp=black (sum 0) → L2 different from
        // Black-to-move: own=black (sum 512), opp=white (sum 0).
        assert_ne!(
            white_result2, black_result2,
            "Perspective swap with asymmetric weights should differ"
        );
    }
}
