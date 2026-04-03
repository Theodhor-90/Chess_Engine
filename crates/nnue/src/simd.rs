use crate::arch::CLIPPED_RELU_MAX;

/// Applies clipped-ReLU: clamps each `i16` to `[0, 127]` and stores as `i8`.
pub(crate) fn clipped_relu_i16_to_i8(input: &[i16], output: &mut [i8]) {
    debug_assert_eq!(input.len(), output.len());

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
    {
        clipped_relu_avx2(input, output);
    }

    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "sse2",
        feature = "simd-sse2",
        not(all(target_feature = "avx2", feature = "simd-avx2"))
    ))]
    {
        clipped_relu_sse2(input, output);
    }

    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        feature = "simd-neon",
        not(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2")),
        not(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))
    ))]
    {
        clipped_relu_neon(input, output);
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"),
        all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"),
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            feature = "simd-neon"
        )
    )))]
    {
        clipped_relu_scalar(input, output);
    }
}

/// Dot product of two `i8` slices, accumulated into `i32`.
pub(crate) fn dot_i8_i32(a: &[i8], b: &[i8]) -> i32 {
    debug_assert_eq!(a.len(), b.len());

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
    {
        dot_avx2(a, b)
    }

    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "sse2",
        feature = "simd-sse2",
        not(all(target_feature = "avx2", feature = "simd-avx2"))
    ))]
    {
        dot_sse2(a, b)
    }

    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        feature = "simd-neon",
        not(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2")),
        not(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))
    ))]
    {
        dot_neon(a, b)
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"),
        all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"),
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            feature = "simd-neon"
        )
    )))]
    {
        dot_scalar(a, b)
    }
}

/// Element-wise add: `acc[i] += weights[i]`.
#[allow(dead_code)]
pub(crate) fn vec_add_i16(acc: &mut [i16], weights: &[i16]) {
    debug_assert_eq!(acc.len(), weights.len());

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
    {
        vec_add_avx2(acc, weights);
    }

    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "sse2",
        feature = "simd-sse2",
        not(all(target_feature = "avx2", feature = "simd-avx2"))
    ))]
    {
        vec_add_sse2(acc, weights);
    }

    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        feature = "simd-neon",
        not(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2")),
        not(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))
    ))]
    {
        vec_add_neon(acc, weights);
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"),
        all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"),
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            feature = "simd-neon"
        )
    )))]
    {
        vec_add_scalar(acc, weights);
    }
}

/// Element-wise subtract: `acc[i] -= weights[i]`.
#[allow(dead_code)]
pub(crate) fn vec_sub_i16(acc: &mut [i16], weights: &[i16]) {
    debug_assert_eq!(acc.len(), weights.len());

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
    {
        vec_sub_avx2(acc, weights);
    }

    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "sse2",
        feature = "simd-sse2",
        not(all(target_feature = "avx2", feature = "simd-avx2"))
    ))]
    {
        vec_sub_sse2(acc, weights);
    }

    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        feature = "simd-neon",
        not(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2")),
        not(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))
    ))]
    {
        vec_sub_neon(acc, weights);
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"),
        all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"),
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            feature = "simd-neon"
        )
    )))]
    {
        vec_sub_scalar(acc, weights);
    }
}

// ---------------------------------------------------------------------------
// Scalar fallbacks
// ---------------------------------------------------------------------------

fn clipped_relu_scalar(input: &[i16], output: &mut [i8]) {
    for (&inp, out) in input.iter().zip(output.iter_mut()) {
        *out = inp.clamp(0, CLIPPED_RELU_MAX) as i8;
    }
}

fn dot_scalar(a: &[i8], b: &[i8]) -> i32 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32) * (y as i32))
        .sum()
}

#[allow(dead_code)]
fn vec_add_scalar(acc: &mut [i16], weights: &[i16]) {
    for (a, &w) in acc.iter_mut().zip(weights.iter()) {
        *a += w;
    }
}

#[allow(dead_code)]
fn vec_sub_scalar(acc: &mut [i16], weights: &[i16]) {
    for (a, &w) in acc.iter_mut().zip(weights.iter()) {
        *a -= w;
    }
}

// ---------------------------------------------------------------------------
// AVX2 implementations
// ---------------------------------------------------------------------------

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
fn clipped_relu_avx2(input: &[i16], output: &mut [i8]) {
    use std::arch::x86_64::*;
    let zero = unsafe { _mm256_setzero_si256() };
    let max_val = unsafe { _mm256_set1_epi16(CLIPPED_RELU_MAX) };
    let chunks = input.len() / 32;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*32+16 <= input.len() because chunks = len/32.
            // Alignment not required for loadu. target_feature = "avx2" guaranteed by cfg.
            let lo = _mm256_loadu_si256(input.as_ptr().add(i * 32) as *const __m256i);
            let hi = _mm256_loadu_si256(input.as_ptr().add(i * 32 + 16) as *const __m256i);
            let lo_clamped = _mm256_min_epi16(_mm256_max_epi16(lo, zero), max_val);
            let hi_clamped = _mm256_min_epi16(_mm256_max_epi16(hi, zero), max_val);
            let packed = _mm256_packs_epi16(lo_clamped, hi_clamped);
            // packs_epi16 interleaves lanes, need permute to fix ordering
            let result = _mm256_permute4x64_epi64(packed, 0b11_01_10_00);
            _mm256_storeu_si256(output.as_mut_ptr().add(i * 32) as *mut __m256i, result);
        }
    }
    let remainder = chunks * 32;
    clipped_relu_scalar(&input[remainder..], &mut output[remainder..]);
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
fn dot_avx2(a: &[i8], b: &[i8]) -> i32 {
    use std::arch::x86_64::*;
    let mut sum = unsafe { _mm256_setzero_si256() };
    let chunks = a.len() / 32;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*32+32 <= chunks*32 <= a.len(). target_feature = "avx2" guaranteed by cfg.
            let va = _mm256_loadu_si256(a.as_ptr().add(i * 32) as *const __m256i);
            let vb = _mm256_loadu_si256(b.as_ptr().add(i * 32) as *const __m256i);
            // _mm256_maddubs_epi16 treats first arg as unsigned, second as signed.
            // Reinterpret: convert signed*signed by splitting into positive/negative parts.
            let sign_a = _mm256_cmpgt_epi8(_mm256_setzero_si256(), va);
            let abs_a = _mm256_sub_epi8(_mm256_xor_si256(va, sign_a), sign_a);
            let prod_pos = _mm256_maddubs_epi16(abs_a, vb);
            let prod_neg = _mm256_sign_epi16(prod_pos, _mm256_or_si256(va, _mm256_set1_epi8(1)));
            sum = _mm256_add_epi32(sum, _mm256_madd_epi16(prod_neg, _mm256_set1_epi16(1)));
        }
    }
    let mut result = unsafe {
        // SAFETY: Extracting 32-bit lanes from sum. target_feature guaranteed by cfg.
        let hi128 = _mm256_extracti128_si256(sum, 1);
        let lo128 = _mm256_castsi256_si128(sum);
        let sum128 = _mm_add_epi32(lo128, hi128);
        let hi64 = _mm_unpackhi_epi64(sum128, sum128);
        let sum64 = _mm_add_epi32(sum128, hi64);
        let hi32 = _mm_shuffle_epi32(sum64, 1);
        let sum32 = _mm_add_epi32(sum64, hi32);
        _mm_cvtsi128_si32(sum32)
    };
    let remainder = chunks * 32;
    result += dot_scalar(&a[remainder..], &b[remainder..]);
    result
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
fn vec_add_avx2(acc: &mut [i16], weights: &[i16]) {
    use std::arch::x86_64::*;
    let chunks = acc.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+16 <= chunks*16 <= acc.len(). target_feature guaranteed by cfg.
            let a = _mm256_loadu_si256(acc.as_ptr().add(i * 16) as *const __m256i);
            let w = _mm256_loadu_si256(weights.as_ptr().add(i * 16) as *const __m256i);
            let result = _mm256_add_epi16(a, w);
            _mm256_storeu_si256(acc.as_mut_ptr().add(i * 16) as *mut __m256i, result);
        }
    }
    let remainder = chunks * 16;
    vec_add_scalar(&mut acc[remainder..], &weights[remainder..]);
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", feature = "simd-avx2"))]
fn vec_sub_avx2(acc: &mut [i16], weights: &[i16]) {
    use std::arch::x86_64::*;
    let chunks = acc.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+16 <= chunks*16 <= acc.len(). target_feature guaranteed by cfg.
            let a = _mm256_loadu_si256(acc.as_ptr().add(i * 16) as *const __m256i);
            let w = _mm256_loadu_si256(weights.as_ptr().add(i * 16) as *const __m256i);
            let result = _mm256_sub_epi16(a, w);
            _mm256_storeu_si256(acc.as_mut_ptr().add(i * 16) as *mut __m256i, result);
        }
    }
    let remainder = chunks * 16;
    vec_sub_scalar(&mut acc[remainder..], &weights[remainder..]);
}

// ---------------------------------------------------------------------------
// SSE2 implementations
// ---------------------------------------------------------------------------

#[cfg(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))]
fn clipped_relu_sse2(input: &[i16], output: &mut [i8]) {
    use std::arch::x86_64::*;
    let zero = unsafe { _mm_setzero_si128() };
    let max_val = unsafe { _mm_set1_epi16(CLIPPED_RELU_MAX) };
    let chunks = input.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+8 <= input.len(). Unaligned loads. target_feature guaranteed by cfg.
            let lo = _mm_loadu_si128(input.as_ptr().add(i * 16) as *const __m128i);
            let hi = _mm_loadu_si128(input.as_ptr().add(i * 16 + 8) as *const __m128i);
            let lo_clamped = _mm_min_epi16(_mm_max_epi16(lo, zero), max_val);
            let hi_clamped = _mm_min_epi16(_mm_max_epi16(hi, zero), max_val);
            let packed = _mm_packs_epi16(lo_clamped, hi_clamped);
            _mm_storeu_si128(output.as_mut_ptr().add(i * 16) as *mut __m128i, packed);
        }
    }
    let remainder = chunks * 16;
    clipped_relu_scalar(&input[remainder..], &mut output[remainder..]);
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))]
fn dot_sse2(a: &[i8], b: &[i8]) -> i32 {
    use std::arch::x86_64::*;
    let mut sum = unsafe { _mm_setzero_si128() };
    let zero = unsafe { _mm_setzero_si128() };
    let chunks = a.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+16 <= a.len(). target_feature guaranteed by cfg.
            let va = _mm_loadu_si128(a.as_ptr().add(i * 16) as *const __m128i);
            let vb = _mm_loadu_si128(b.as_ptr().add(i * 16) as *const __m128i);
            // Pure SSE2: widen i8 to i16 via sign extension, then use _mm_madd_epi16.
            // Low 8 bytes:
            let a_lo = _mm_unpacklo_epi8(va, _mm_cmpgt_epi8(zero, va));
            let b_lo = _mm_unpacklo_epi8(vb, _mm_cmpgt_epi8(zero, vb));
            sum = _mm_add_epi32(sum, _mm_madd_epi16(a_lo, b_lo));
            // High 8 bytes:
            let a_hi = _mm_unpackhi_epi8(va, _mm_cmpgt_epi8(zero, va));
            let b_hi = _mm_unpackhi_epi8(vb, _mm_cmpgt_epi8(zero, vb));
            sum = _mm_add_epi32(sum, _mm_madd_epi16(a_hi, b_hi));
        }
    }
    let mut result = unsafe {
        // SAFETY: Extracting 32-bit lanes. target_feature guaranteed by cfg.
        let hi64 = _mm_unpackhi_epi64(sum, sum);
        let sum64 = _mm_add_epi32(sum, hi64);
        let hi32 = _mm_shuffle_epi32(sum64, 1);
        let sum32 = _mm_add_epi32(sum64, hi32);
        _mm_cvtsi128_si32(sum32)
    };
    let remainder = chunks * 16;
    result += dot_scalar(&a[remainder..], &b[remainder..]);
    result
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))]
fn vec_add_sse2(acc: &mut [i16], weights: &[i16]) {
    use std::arch::x86_64::*;
    let chunks = acc.len() / 8;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*8+8 <= acc.len(). target_feature guaranteed by cfg.
            let a = _mm_loadu_si128(acc.as_ptr().add(i * 8) as *const __m128i);
            let w = _mm_loadu_si128(weights.as_ptr().add(i * 8) as *const __m128i);
            let result = _mm_add_epi16(a, w);
            _mm_storeu_si128(acc.as_mut_ptr().add(i * 8) as *mut __m128i, result);
        }
    }
    let remainder = chunks * 8;
    vec_add_scalar(&mut acc[remainder..], &weights[remainder..]);
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2", feature = "simd-sse2"))]
fn vec_sub_sse2(acc: &mut [i16], weights: &[i16]) {
    use std::arch::x86_64::*;
    let chunks = acc.len() / 8;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*8+8 <= acc.len(). target_feature guaranteed by cfg.
            let a = _mm_loadu_si128(acc.as_ptr().add(i * 8) as *const __m128i);
            let w = _mm_loadu_si128(weights.as_ptr().add(i * 8) as *const __m128i);
            let result = _mm_sub_epi16(a, w);
            _mm_storeu_si128(acc.as_mut_ptr().add(i * 8) as *mut __m128i, result);
        }
    }
    let remainder = chunks * 8;
    vec_sub_scalar(&mut acc[remainder..], &weights[remainder..]);
}

// ---------------------------------------------------------------------------
// NEON implementations
// ---------------------------------------------------------------------------

#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    feature = "simd-neon"
))]
fn clipped_relu_neon(input: &[i16], output: &mut [i8]) {
    use std::arch::aarch64::*;
    let zero = unsafe { vdupq_n_s16(0) };
    let max_val = unsafe { vdupq_n_s16(CLIPPED_RELU_MAX) };
    let chunks = input.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+8 <= input.len(). target_feature guaranteed by cfg.
            let lo = vld1q_s16(input.as_ptr().add(i * 16));
            let hi = vld1q_s16(input.as_ptr().add(i * 16 + 8));
            let lo_clamped = vminq_s16(vmaxq_s16(lo, zero), max_val);
            let hi_clamped = vminq_s16(vmaxq_s16(hi, zero), max_val);
            let lo_narrow = vqmovn_s16(lo_clamped);
            let hi_narrow = vqmovn_s16(hi_clamped);
            let combined = vcombine_s8(lo_narrow, hi_narrow);
            vst1q_s8(output.as_mut_ptr().add(i * 16), combined);
        }
    }
    let remainder = chunks * 16;
    clipped_relu_scalar(&input[remainder..], &mut output[remainder..]);
}

#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    feature = "simd-neon"
))]
fn dot_neon(a: &[i8], b: &[i8]) -> i32 {
    use std::arch::aarch64::*;
    let mut sum = unsafe { vdupq_n_s32(0) };
    let chunks = a.len() / 16;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*16+16 <= a.len(). target_feature guaranteed by cfg.
            let va = vld1q_s8(a.as_ptr().add(i * 16));
            let vb = vld1q_s8(b.as_ptr().add(i * 16));
            let lo_a = vget_low_s8(va);
            let hi_a = vget_high_s8(va);
            let lo_b = vget_low_s8(vb);
            let hi_b = vget_high_s8(vb);
            let prod_lo = vmull_s8(lo_a, lo_b);
            let prod_hi = vmull_s8(hi_a, hi_b);
            sum = vpadalq_s16(sum, prod_lo);
            sum = vpadalq_s16(sum, prod_hi);
        }
    }
    let mut result = unsafe { vaddvq_s32(sum) };
    let remainder = chunks * 16;
    result += dot_scalar(&a[remainder..], &b[remainder..]);
    result
}

#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    feature = "simd-neon"
))]
fn vec_add_neon(acc: &mut [i16], weights: &[i16]) {
    use std::arch::aarch64::*;
    let chunks = acc.len() / 8;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*8+8 <= acc.len(). target_feature guaranteed by cfg.
            let a = vld1q_s16(acc.as_ptr().add(i * 8));
            let w = vld1q_s16(weights.as_ptr().add(i * 8));
            let result = vaddq_s16(a, w);
            vst1q_s16(acc.as_mut_ptr().add(i * 8), result);
        }
    }
    let remainder = chunks * 8;
    vec_add_scalar(&mut acc[remainder..], &weights[remainder..]);
}

#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    feature = "simd-neon"
))]
fn vec_sub_neon(acc: &mut [i16], weights: &[i16]) {
    use std::arch::aarch64::*;
    let chunks = acc.len() / 8;
    for i in 0..chunks {
        unsafe {
            // SAFETY: i*8+8 <= acc.len(). target_feature guaranteed by cfg.
            let a = vld1q_s16(acc.as_ptr().add(i * 8));
            let w = vld1q_s16(weights.as_ptr().add(i * 8));
            let result = vsubq_s16(a, w);
            vst1q_s16(acc.as_mut_ptr().add(i * 8), result);
        }
    }
    let remainder = chunks * 8;
    vec_sub_scalar(&mut acc[remainder..], &weights[remainder..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipped_relu_scalar_reference() {
        let input: Vec<i16> = (-500..=500).collect();
        let mut output = vec![0i8; input.len()];
        clipped_relu_i16_to_i8(&input, &mut output);
        for (inp, &out) in input.iter().zip(output.iter()) {
            let expected = (*inp).clamp(0, CLIPPED_RELU_MAX) as i8;
            assert_eq!(out, expected, "Failed for input {inp}");
        }
    }

    #[test]
    fn dot_product_scalar_reference() {
        let a: Vec<i8> = (0..64).map(|i| (i * 3 - 90) as i8).collect();
        let b: Vec<i8> = (0..64).map(|i| (i * 2 - 60) as i8).collect();
        let result = dot_i8_i32(&a, &b);
        let expected: i32 = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x as i32) * (y as i32))
            .sum();
        assert_eq!(result, expected);
    }

    #[test]
    fn vec_add_sub_roundtrip() {
        let mut acc = vec![10i16; 256];
        let weights: Vec<i16> = (0..256).map(|i| (i * 5 - 600) as i16).collect();
        let original = acc.clone();
        vec_add_i16(&mut acc, &weights);
        assert_ne!(acc, original);
        vec_sub_i16(&mut acc, &weights);
        assert_eq!(acc, original);
    }

    #[test]
    fn simd_matches_scalar() {
        // clipped_relu
        let input: Vec<i16> = (-300..300).collect();
        let mut output_dispatch = vec![0i8; input.len()];
        let mut output_scalar = vec![0i8; input.len()];
        clipped_relu_i16_to_i8(&input, &mut output_dispatch);
        clipped_relu_scalar(&input, &mut output_scalar);
        assert_eq!(output_dispatch, output_scalar);

        // dot product
        let a: Vec<i8> = (0..128).map(|i| (i * 2 - 127) as i8).collect();
        let b: Vec<i8> = (0..128).map(|i| (i - 64) as i8).collect();
        let dot_dispatch = dot_i8_i32(&a, &b);
        let dot_scalar_val = dot_scalar(&a, &b);
        assert_eq!(dot_dispatch, dot_scalar_val);

        // vec_add
        let mut acc_dispatch: Vec<i16> = (0..256).map(|i| i as i16).collect();
        let mut acc_scalar = acc_dispatch.clone();
        let weights: Vec<i16> = (0..256).map(|i| (i * 3 - 400) as i16).collect();
        vec_add_i16(&mut acc_dispatch, &weights);
        vec_add_scalar(&mut acc_scalar, &weights);
        assert_eq!(acc_dispatch, acc_scalar);

        // vec_sub
        let mut acc_dispatch: Vec<i16> = (0..256).map(|i| i as i16).collect();
        let mut acc_scalar = acc_dispatch.clone();
        vec_sub_i16(&mut acc_dispatch, &weights);
        vec_sub_scalar(&mut acc_scalar, &weights);
        assert_eq!(acc_dispatch, acc_scalar);
    }
}
