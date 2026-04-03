use chess_types::Color;

use crate::accumulator::Accumulator;
use crate::arch::QUANTIZATION_SHIFT_HIDDEN;
use crate::network::Network;
use crate::simd;

pub fn forward(acc: &Accumulator, network: &Network, side_to_move: Color) -> i32 {
    let l1_size = network.dims().l1_size;
    let l2_size = network.dims().l2_size;

    let (own, opp) = match side_to_move {
        Color::White => (&acc.white, &acc.black),
        Color::Black => (&acc.black, &acc.white),
    };

    let mut l1_input = vec![0i16; 2 * l1_size];
    l1_input[..l1_size].copy_from_slice(own);
    l1_input[l1_size..].copy_from_slice(opp);

    let mut l1_output = vec![0i8; 2 * l1_size];
    clipped_relu(&l1_input, &mut l1_output);

    let mut l2_values = vec![0i32; l2_size];
    let input_len = 2 * l1_size;
    for (row, l2_val) in l2_values.iter_mut().enumerate() {
        let row_offset = row * input_len;
        let row_weights = &network.hidden1_weights[row_offset..row_offset + input_len];
        *l2_val = simd::dot_i8_i32(row_weights, &l1_output) + network.hidden1_bias[row];
    }

    let mut l2_rescaled = vec![0i16; l2_size];
    for i in 0..l2_size {
        l2_rescaled[i] = (l2_values[i] >> QUANTIZATION_SHIFT_HIDDEN) as i16;
    }
    let mut l2_output = vec![0i8; l2_size];
    clipped_relu(&l2_rescaled, &mut l2_output);

    let output = simd::dot_i8_i32(&network.hidden2_weights, &l2_output) + network.hidden2_bias;

    output >> QUANTIZATION_SHIFT_HIDDEN
}

fn clipped_relu(input: &[i16], output: &mut [i8]) {
    simd::clipped_relu_i16_to_i8(input, output);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::{NetworkDims, L1_SIZE};

    #[test]
    fn forward_zeroed_network() {
        let network = Network::new_zeroed(NetworkDims::default_full());
        let acc = Accumulator::new(L1_SIZE);
        let result = forward(&acc, &network, Color::White);
        assert_eq!(result, 0);
    }

    #[test]
    fn forward_known_reference() {
        let mut network = Network::new_zeroed(NetworkDims::default_full());

        for w in network.hidden1_weights.iter_mut() {
            *w = 1;
        }
        for w in network.hidden2_weights.iter_mut() {
            *w = 1;
        }

        let mut acc = Accumulator::new(L1_SIZE);
        for i in 0..L1_SIZE {
            acc.white[i] = i as i16;
            acc.black[i] = (L1_SIZE - 1 - i) as i16;
        }

        let result = forward(&acc, &network, Color::White);
        assert_eq!(result, 63);
    }

    #[test]
    fn forward_perspective_swap() {
        let mut network = Network::new_zeroed(NetworkDims::default_full());
        for w in network.hidden1_weights.iter_mut() {
            *w = 1;
        }
        for w in network.hidden2_weights.iter_mut() {
            *w = 1;
        }

        let mut acc = Accumulator::new(L1_SIZE);
        for i in 0..L1_SIZE {
            acc.white[i] = (i as i16) % 100;
            acc.black[i] = ((i as i16) % 100) + 10;
        }

        let white_result = forward(&acc, &network, Color::White);
        let black_result = forward(&acc, &network, Color::Black);

        assert_eq!(white_result, black_result);

        let mut acc2 = Accumulator::new(L1_SIZE);
        for i in 0..L1_SIZE {
            acc2.white[i] = 1;
            acc2.black[i] = 2;
        }

        let mut network2 = Network::new_zeroed(NetworkDims::default_full());
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

        assert_ne!(
            white_result2, black_result2,
            "Perspective swap with asymmetric weights should differ"
        );
    }

    #[test]
    fn forward_non_default_dimensions() {
        let l1 = 128;
        let l2 = 16;
        let dims = NetworkDims {
            halfkp_features: 40960,
            l1_size: l1,
            l2_size: l2,
            output_size: 1,
        };
        let mut network = Network::new_zeroed(dims);

        for w in network.hidden1_weights.iter_mut() {
            *w = 1;
        }
        for w in network.hidden2_weights.iter_mut() {
            *w = 1;
        }

        let mut acc = Accumulator::new(l1);
        for i in 0..l1 {
            acc.white[i] = i as i16;
            acc.black[i] = (l1 - 1 - i) as i16;
        }

        let result = forward(&acc, &network, Color::White);

        // Hand-compute:
        // L1 input = [white, black] (256 i16 values).
        // After clipped-ReLU: white[i] = min(i, 127), black[i] = min(127-i, 127)
        // For white: i in 0..128 => 0,1,...,127 sum = 127*128/2 = 8128
        // For black: 127-i for i in 0..128 => 127,126,...,0 sum = 8128
        // Dot with all-1 hidden1_weights for each L2 row:
        //   Each row gets 8128 + 8128 = 16256
        //   l2_values[row] = 16256 + 0 (bias) = 16256
        // Rescaled: 16256 >> 6 = 254
        // Clipped-ReLU: min(254, 127) = 127
        // L2 output: all 16 values = 127
        // Dot product with hidden2_weights (all 1s): 127 * 16 = 2032
        // Plus hidden2_bias (0): 2032
        // Final: 2032 >> 6 = 31
        assert_eq!(result, 31);
    }
}
