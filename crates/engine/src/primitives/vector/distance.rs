//! Shared distance functions for vector similarity computation.
//!
//! These functions are used by both BruteForceBackend and HnswBackend.
//!
//! All scores are normalized to "higher = more similar" (Invariant R2).
//! Functions are single-threaded for determinism (Invariant R8).
//! No implicit normalization of vectors (Invariant R9).
//!
//! When available, AVX2+FMA SIMD intrinsics accelerate distance computation
//! (~4-8x speedup on 384-dim vectors). Falls back to scalar code on other
//! architectures.

use crate::primitives::vector::DistanceMetric;

/// Compute similarity score between two vectors
///
/// All scores are normalized to "higher = more similar" (Invariant R2).
/// This function is single-threaded for determinism (Invariant R8).
///
/// IMPORTANT: No implicit normalization of vectors (Invariant R9).
/// Vectors are used as-is.
pub fn compute_similarity(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    debug_assert_eq!(
        a.len(),
        b.len(),
        "Dimension mismatch in similarity computation"
    );

    match metric {
        DistanceMetric::Cosine => cosine_similarity(a, b),
        DistanceMetric::Euclidean => euclidean_similarity(a, b),
        DistanceMetric::DotProduct => dot_product(a, b),
    }
}

// ============================================================================
// SIMD (AVX2 + FMA) fast paths — x86_64 only
// ============================================================================

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Horizontal sum of an __m256 register → f32
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
#[inline]
unsafe fn hsum_ps_avx2(v: __m256) -> f32 {
    // v = [a0, a1, a2, a3, a4, a5, a6, a7]
    let hi128 = _mm256_extractf128_ps(v, 1); // [a4, a5, a6, a7]
    let lo128 = _mm256_castps256_ps128(v); // [a0, a1, a2, a3]
    let sum128 = _mm_add_ps(lo128, hi128); // [a0+a4, a1+a5, a2+a6, a3+a7]
    let hi64 = _mm_movehl_ps(sum128, sum128); // [a2+a6, a3+a7, ...]
    let sum64 = _mm_add_ps(sum128, hi64); // [a0+a2+a4+a6, a1+a3+a5+a7, ...]
    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x1); // [a1+a3+a5+a7, ...]
    let sum32 = _mm_add_ss(sum64, hi32);
    _mm_cvtss_f32(sum32)
}

/// AVX2+FMA dot product: processes 8 floats per iteration
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 8;
    let remainder = n % 8;

    let mut sum = _mm256_setzero_ps();

    let pa = a.as_ptr();
    let pb = b.as_ptr();

    for i in 0..chunks {
        let va = _mm256_loadu_ps(pa.add(i * 8));
        let vb = _mm256_loadu_ps(pb.add(i * 8));
        sum = _mm256_fmadd_ps(va, vb, sum);
    }

    let mut result = hsum_ps_avx2(sum);

    // Scalar remainder
    let base = chunks * 8;
    for i in 0..remainder {
        result += *pa.add(base + i) * *pb.add(base + i);
    }

    result
}

/// AVX2+FMA fused cosine similarity: single pass computing dot(a,b), ||a||², ||b||²
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn cosine_similarity_avx2(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 8;
    let remainder = n % 8;

    let mut dot_sum = _mm256_setzero_ps();
    let mut norm_a_sum = _mm256_setzero_ps();
    let mut norm_b_sum = _mm256_setzero_ps();

    let pa = a.as_ptr();
    let pb = b.as_ptr();

    for i in 0..chunks {
        let va = _mm256_loadu_ps(pa.add(i * 8));
        let vb = _mm256_loadu_ps(pb.add(i * 8));
        dot_sum = _mm256_fmadd_ps(va, vb, dot_sum);
        norm_a_sum = _mm256_fmadd_ps(va, va, norm_a_sum);
        norm_b_sum = _mm256_fmadd_ps(vb, vb, norm_b_sum);
    }

    let mut dot = hsum_ps_avx2(dot_sum);
    let mut norm_a_sq = hsum_ps_avx2(norm_a_sum);
    let mut norm_b_sq = hsum_ps_avx2(norm_b_sum);

    // Scalar remainder
    let base = chunks * 8;
    for i in 0..remainder {
        let ai = *pa.add(base + i);
        let bi = *pb.add(base + i);
        dot += ai * bi;
        norm_a_sq += ai * ai;
        norm_b_sq += bi * bi;
    }

    let norm_a = norm_a_sq.sqrt();
    let norm_b = norm_b_sq.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// AVX2+FMA euclidean distance: sum of squared differences
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn euclidean_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 8;
    let remainder = n % 8;

    let mut sum = _mm256_setzero_ps();

    let pa = a.as_ptr();
    let pb = b.as_ptr();

    for i in 0..chunks {
        let va = _mm256_loadu_ps(pa.add(i * 8));
        let vb = _mm256_loadu_ps(pb.add(i * 8));
        let diff = _mm256_sub_ps(va, vb);
        sum = _mm256_fmadd_ps(diff, diff, sum);
    }

    let mut result = hsum_ps_avx2(sum);

    // Scalar remainder
    let base = chunks * 8;
    for i in 0..remainder {
        let d = *pa.add(base + i) - *pb.add(base + i);
        result += d * d;
    }

    result.sqrt()
}

// ============================================================================
// Runtime dispatch: use SIMD when available, scalar fallback otherwise
// ============================================================================

/// Cosine similarity: dot(a,b) / (||a|| * ||b||)
///
/// Range: [-1, 1], higher = more similar
/// Returns 0.0 if either vector has zero norm (avoids division by zero)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return unsafe { cosine_similarity_avx2(a, b) };
        }
    }
    cosine_similarity_scalar(a, b)
}

/// Euclidean similarity: 1 / (1 + l2_distance)
///
/// Range: (0, 1], higher = more similar
/// Transforms distance to similarity (inversely related)
fn euclidean_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dist = euclidean_distance(a, b);
    1.0 / (1.0 + dist)
}

/// Dot product (inner product)
///
/// Range: unbounded, higher = more similar
/// Assumes vectors are pre-normalized for meaningful comparison
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return unsafe { dot_product_avx2(a, b) };
        }
    }
    dot_product_scalar(a, b)
}

/// Euclidean distance (L2 distance)
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return unsafe { euclidean_distance_avx2(a, b) };
        }
    }
    euclidean_distance_scalar(a, b)
}

// ============================================================================
// Scalar fallback implementations
// ============================================================================

fn cosine_similarity_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a_sq = 0.0f32;
    let mut norm_b_sq = 0.0f32;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot += ai * bi;
        norm_a_sq += ai * ai;
        norm_b_sq += bi * bi;
    }

    let norm_a = norm_a_sq.sqrt();
    let norm_b = norm_b_sq.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn euclidean_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![-1.0, 0.0];
        let sim = cosine_similarity(&v1, &v2);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = euclidean_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distant_vectors() {
        let v1 = vec![0.0, 0.0];
        let v2 = vec![100.0, 0.0];
        let sim = euclidean_similarity(&v1, &v2);
        assert!(sim < 0.01);
        assert!(sim > 0.0);
        assert!(sim <= 1.0);
    }

    #[test]
    fn test_dot_product_unit_vectors() {
        let v = vec![1.0, 0.0];
        assert!((dot_product(&v, &v) - 1.0).abs() < 1e-6);

        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        assert!(dot_product(&v1, &v2).abs() < 1e-6);
    }

    #[test]
    fn test_zero_vector_handling() {
        let zero = vec![0.0, 0.0, 0.0];
        let nonzero = vec![1.0, 2.0, 3.0];

        assert_eq!(cosine_similarity(&zero, &nonzero), 0.0);
        assert_eq!(cosine_similarity(&nonzero, &zero), 0.0);
        assert_eq!(cosine_similarity(&zero, &zero), 0.0);

        let sim = euclidean_similarity(&zero, &nonzero);
        assert!(sim > 0.0 && sim <= 1.0);
    }

    #[test]
    fn test_compute_similarity_dispatches_correctly() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let cosine = compute_similarity(&a, &b, DistanceMetric::Cosine);
        assert!(cosine.abs() < 1e-6); // Orthogonal

        let euclidean = compute_similarity(&a, &b, DistanceMetric::Euclidean);
        assert!(euclidean > 0.0 && euclidean < 1.0);

        let dot = compute_similarity(&a, &b, DistanceMetric::DotProduct);
        assert!(dot.abs() < 1e-6); // Orthogonal
    }

    #[test]
    fn test_simd_matches_scalar_384dim() {
        // Test with 384-dim vectors (MiniLM dimension) to verify SIMD path
        let a: Vec<f32> = (0..384).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = (0..384).map(|i| ((384 - i) as f32) * 0.01).collect();

        let scalar_dot = dot_product_scalar(&a, &b);
        let fast_dot = dot_product(&a, &b);
        assert!(
            (scalar_dot - fast_dot).abs() < 1e-3,
            "dot mismatch: scalar={scalar_dot}, fast={fast_dot}"
        );

        let scalar_cos = cosine_similarity_scalar(&a, &b);
        let fast_cos = cosine_similarity(&a, &b);
        assert!(
            (scalar_cos - fast_cos).abs() < 1e-5,
            "cosine mismatch: scalar={scalar_cos}, fast={fast_cos}"
        );

        let scalar_euc = euclidean_distance_scalar(&a, &b);
        let fast_euc = euclidean_distance(&a, &b);
        assert!(
            (scalar_euc - fast_euc).abs() < 1e-3,
            "euclidean mismatch: scalar={scalar_euc}, fast={fast_euc}"
        );
    }

    #[test]
    fn test_simd_matches_scalar_negative_values() {
        // Test with negative values to verify sign handling in SIMD paths
        let a: Vec<f32> = (0..384)
            .map(|i| if i % 2 == 0 { (i as f32) * 0.01 } else { -(i as f32) * 0.01 })
            .collect();
        let b: Vec<f32> = (0..384)
            .map(|i| if i % 3 == 0 { -(i as f32) * 0.02 } else { (i as f32) * 0.005 })
            .collect();

        let scalar_dot = dot_product_scalar(&a, &b);
        let fast_dot = dot_product(&a, &b);
        assert!(
            (scalar_dot - fast_dot).abs() < 1e-2,
            "dot mismatch with negatives: scalar={scalar_dot}, fast={fast_dot}"
        );

        let scalar_cos = cosine_similarity_scalar(&a, &b);
        let fast_cos = cosine_similarity(&a, &b);
        assert!(
            (scalar_cos - fast_cos).abs() < 1e-5,
            "cosine mismatch with negatives: scalar={scalar_cos}, fast={fast_cos}"
        );

        let scalar_euc = euclidean_distance_scalar(&a, &b);
        let fast_euc = euclidean_distance(&a, &b);
        assert!(
            (scalar_euc - fast_euc).abs() < 1e-1,
            "euclidean mismatch with negatives: scalar={scalar_euc}, fast={fast_euc}"
        );
    }

    #[test]
    fn test_simd_odd_dimension() {
        // Test with non-multiple-of-8 dimension to exercise remainder logic
        let a: Vec<f32> = (0..13).map(|i| (i as f32) * 0.1).collect();
        let b: Vec<f32> = (0..13).map(|i| ((13 - i) as f32) * 0.1).collect();

        let scalar_dot = dot_product_scalar(&a, &b);
        let fast_dot = dot_product(&a, &b);
        assert!(
            (scalar_dot - fast_dot).abs() < 1e-5,
            "dot mismatch on odd dim: scalar={scalar_dot}, fast={fast_dot}"
        );

        let scalar_cos = cosine_similarity_scalar(&a, &b);
        let fast_cos = cosine_similarity(&a, &b);
        assert!(
            (scalar_cos - fast_cos).abs() < 1e-5,
            "cosine mismatch on odd dim: scalar={scalar_cos}, fast={fast_cos}"
        );
    }
}
