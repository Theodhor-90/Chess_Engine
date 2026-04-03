/// Number of squares on the board.
pub const NUM_SQUARES: usize = 64;

/// Number of non-king piece kinds in HalfKP (Pawn, Knight, Bishop, Rook, Queen).
pub const NUM_PIECE_KINDS: usize = 5;

/// Number of piece colors.
pub const NUM_COLORS: usize = 2;

/// Total HalfKP features per perspective.
///
/// 64 king squares × 10 colored non-king piece types × 64 piece squares = 40,960.
pub const HALFKP_FEATURES: usize = NUM_SQUARES * NUM_PIECE_KINDS * NUM_COLORS * NUM_SQUARES;

/// Input feature dimensionality per perspective.
pub const INPUT_SIZE: usize = HALFKP_FEATURES;

/// Hidden layer 1 (accumulator) output dimension.
pub const L1_SIZE: usize = 256;

/// Hidden layer 2 output dimension.
pub const L2_SIZE: usize = 32;

/// Single scalar output.
pub const OUTPUT_SIZE: usize = 1;

/// Quantization shift for accumulator weights.
///
/// Accumulator (input → L1): weights are `i16`, biases are `i16`.
/// Real-valued weights are quantized by multiplying by `2^QUANTIZATION_SHIFT_ACCUMULATOR`
/// (64) and rounding to the nearest integer. This preserves precision while enabling
/// fast integer arithmetic.
pub const QUANTIZATION_SHIFT_ACCUMULATOR: i32 = 6;

/// Quantization shift for hidden-layer weights.
///
/// Hidden layers (L1 → L2, L2 → output): weights are `i8`, biases are `i32`.
/// The smaller `i8` type reduces memory footprint and enables SIMD-friendly dot products.
/// Biases are `i32` to hold accumulated products without overflow.
pub const QUANTIZATION_SHIFT_HIDDEN: i32 = 6;

/// Clamp range for clipped-ReLU activation output.
///
/// Clipped-ReLU activation clamps values to `[0, CLIPPED_RELU_MAX]` (i.e., `[0, 127]`),
/// fitting the result into an `i8` range for the next layer's input.
pub const CLIPPED_RELU_MAX: i16 = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkDims {
    pub halfkp_features: usize,
    pub l1_size: usize,
    pub l2_size: usize,
    pub output_size: usize,
}

impl NetworkDims {
    pub fn default_full() -> Self {
        Self {
            halfkp_features: HALFKP_FEATURES,
            l1_size: L1_SIZE,
            l2_size: L2_SIZE,
            output_size: OUTPUT_SIZE,
        }
    }
}
