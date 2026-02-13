//! CPU compute backend wrapping existing Tensor operations.
//!
//! This is the default fallback backend. It wraps `Tensor` methods directly,
//! producing bit-identical results to the original non-backend code path.

use super::backend::{ComputeBackend, DeviceTensor};
use super::tensor::Tensor;

/// CPU compute backend — delegates to Tensor methods.
pub struct CpuBackend;

impl CpuBackend {
    fn as_tensor(dt: &DeviceTensor) -> &Tensor {
        dt.inner
            .downcast_ref::<Tensor>()
            .expect("CpuBackend: expected Tensor in DeviceTensor")
    }

    fn as_tensor_mut(dt: &mut DeviceTensor) -> &mut Tensor {
        dt.inner
            .downcast_mut::<Tensor>()
            .expect("CpuBackend: expected Tensor in DeviceTensor")
    }

    fn wrap(t: Tensor) -> DeviceTensor {
        DeviceTensor {
            rows: t.rows,
            cols: t.cols,
            inner: Box::new(t),
        }
    }

    fn as_1d(dt: &DeviceTensor) -> &[f32] {
        let t = Self::as_tensor(dt);
        assert_eq!(t.rows, 1, "expected 1-row tensor for 1D data");
        &t.data
    }
}

impl ComputeBackend for CpuBackend {
    fn upload(&self, t: &Tensor) -> DeviceTensor {
        Self::wrap(t.clone())
    }

    fn upload_1d(&self, v: &[f32]) -> DeviceTensor {
        Self::wrap(Tensor::from_slice(v, 1, v.len()))
    }

    fn download(&self, dt: &DeviceTensor) -> Tensor {
        Self::as_tensor(dt).clone()
    }

    fn matmul(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        Self::wrap(Self::as_tensor(a).matmul(Self::as_tensor(b)))
    }

    fn matmul_transpose(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        Self::wrap(Self::as_tensor(a).matmul_transpose(Self::as_tensor(b)))
    }

    fn add_bias(&self, t: &mut DeviceTensor, bias: &DeviceTensor) {
        let bias_data = Self::as_1d(bias);
        Self::as_tensor_mut(t).add_bias(bias_data);
    }

    fn add_tensor(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        Self::wrap(Self::as_tensor(a).add_tensor(Self::as_tensor(b)))
    }

    fn gelu(&self, t: &DeviceTensor) -> DeviceTensor {
        Self::wrap(Self::as_tensor(t).gelu())
    }

    fn layer_norm(
        &self,
        t: &DeviceTensor,
        w: &DeviceTensor,
        b: &DeviceTensor,
        eps: f32,
    ) -> DeviceTensor {
        let weight = Self::as_1d(w);
        let bias = Self::as_1d(b);
        Self::wrap(Self::as_tensor(t).layer_norm(weight, bias, eps))
    }

    fn softmax_rows(&self, t: &mut DeviceTensor) {
        Self::as_tensor_mut(t).softmax_rows();
    }

    fn scale(&self, t: &mut DeviceTensor, factor: f32) {
        Self::as_tensor_mut(t).scale(factor);
    }

    fn slice_columns(&self, t: &DeviceTensor, start: usize, end: usize) -> DeviceTensor {
        let src = Self::as_tensor(t);
        let width = end - start;
        let mut data = vec![0.0f32; src.rows * width];
        for r in 0..src.rows {
            let src_off = r * src.cols + start;
            let dst_off = r * width;
            data[dst_off..dst_off + width].copy_from_slice(&src.data[src_off..src_off + width]);
        }
        Self::wrap(Tensor::from_slice(&data, src.rows, width))
    }

    fn scatter_columns(&self, dst: &mut DeviceTensor, src: &DeviceTensor, col_offset: usize) {
        let src_t = Self::as_tensor(src);
        let dst_t = Self::as_tensor_mut(dst);
        for r in 0..src_t.rows {
            let src_off = r * src_t.cols;
            let dst_off = r * dst_t.cols + col_offset;
            dst_t.data[dst_off..dst_off + src_t.cols]
                .copy_from_slice(&src_t.data[src_off..src_off + src_t.cols]);
        }
    }

    fn zeros(&self, rows: usize, cols: usize) -> DeviceTensor {
        Self::wrap(Tensor::zeros(rows, cols))
    }

    fn upload_mask(&self, mask: &[u32]) -> DeviceTensor {
        DeviceTensor {
            rows: 1,
            cols: mask.len(),
            inner: Box::new(mask.to_vec()),
        }
    }

    fn apply_attention_mask(&self, scores: &mut DeviceTensor, mask: &DeviceTensor) {
        let mask = mask.inner.downcast_ref::<Vec<u32>>().expect("CpuBackend: expected Vec<u32> mask");
        let t = Self::as_tensor_mut(scores);
        let seq_len = t.cols;
        for i in 0..t.rows {
            for j in 0..seq_len {
                if mask[j] == 0 {
                    t.data[i * seq_len + j] = -10000.0;
                }
            }
        }
    }

    fn mean_pool(&self, hidden: &DeviceTensor, mask: &DeviceTensor) -> Vec<f32> {
        let mask = mask.inner.downcast_ref::<Vec<u32>>().expect("CpuBackend: expected Vec<u32> mask");
        let t = Self::as_tensor(hidden);
        let mut sum = vec![0.0f32; t.cols];
        let mut count = 0.0f32;
        for s in 0..t.rows {
            if mask[s] == 1 {
                let row = t.row(s);
                for i in 0..t.cols {
                    sum[i] += row[i];
                }
                count += 1.0;
            }
        }
        if count > 0.0 {
            for v in sum.iter_mut() {
                *v /= count;
            }
        }
        sum
    }

    fn batched_matmul_transpose(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor {
        let a_t = Self::as_tensor(a);
        let b_t = Self::as_tensor(b);
        let k = a_t.cols;
        let mut data = vec![0.0f32; batch_size * seq_len * seq_len];

        for batch in 0..batch_size {
            let a_off = batch * seq_len * k;
            let b_off = batch * seq_len * k;
            let c_off = batch * seq_len * seq_len;
            for i in 0..seq_len {
                for j in 0..seq_len {
                    let mut sum = 0.0f32;
                    for x in 0..k {
                        sum += a_t.data[a_off + i * k + x] * b_t.data[b_off + j * k + x];
                    }
                    data[c_off + i * seq_len + j] = sum;
                }
            }
        }

        Self::wrap(Tensor::from_slice(&data, batch_size * seq_len, seq_len))
    }

    fn batched_matmul(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor {
        let a_t = Self::as_tensor(a);
        let b_t = Self::as_tensor(b);
        let k = b_t.cols;
        let mut data = vec![0.0f32; batch_size * seq_len * k];

        for batch in 0..batch_size {
            let a_off = batch * seq_len * seq_len;
            let b_off = batch * seq_len * k;
            let c_off = batch * seq_len * k;
            for i in 0..seq_len {
                for j in 0..k {
                    let mut sum = 0.0f32;
                    for x in 0..seq_len {
                        sum += a_t.data[a_off + i * seq_len + x] * b_t.data[b_off + x * k + j];
                    }
                    data[c_off + i * k + j] = sum;
                }
            }
        }

        Self::wrap(Tensor::from_slice(&data, batch_size * seq_len, k))
    }

    fn batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) {
        let mask = mask.inner.downcast_ref::<Vec<u32>>().expect("CpuBackend: expected Vec<u32> mask");
        let t = Self::as_tensor_mut(scores);
        for row in 0..(batch_size * seq_len) {
            let batch = row / seq_len;
            for col in 0..seq_len {
                if mask[batch * seq_len + col] == 0 {
                    t.data[row * seq_len + col] = -10000.0;
                }
            }
        }
    }

    fn multi_head_batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
    ) {
        let mask = mask
            .inner
            .downcast_ref::<Vec<u32>>()
            .expect("CpuBackend: expected Vec<u32> mask");
        let t = Self::as_tensor_mut(scores);
        let group_size = num_heads * seq_len;
        let total_rows = batch_size * group_size;
        for row in 0..total_rows {
            let batch = row / group_size;
            for col in 0..seq_len {
                if mask[batch * seq_len + col] == 0 {
                    t.data[row * t.cols + col] = -10000.0;
                }
            }
        }
    }

    fn batched_mean_pool(
        &self,
        hidden: &DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> Vec<Vec<f32>> {
        let mask = mask.inner.downcast_ref::<Vec<u32>>().expect("CpuBackend: expected Vec<u32> mask");
        let t = Self::as_tensor(hidden);
        let d = t.cols;
        let mut results = Vec::with_capacity(batch_size);

        for batch in 0..batch_size {
            let mut sum = vec![0.0f32; d];
            let mut count = 0.0f32;
            for s in 0..seq_len {
                let idx = batch * seq_len + s;
                if mask[idx] == 1 {
                    let row = &t.data[idx * d..(idx + 1) * d];
                    for i in 0..d {
                        sum[i] += row[i];
                    }
                    count += 1.0;
                }
            }
            if count > 0.0 {
                for v in sum.iter_mut() {
                    *v /= count;
                }
            }
            results.push(sum);
        }

        results
    }

    fn name(&self) -> &'static str {
        "CPU"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backend::ComputeBackend;

    fn backend() -> CpuBackend {
        CpuBackend
    }

    // -------------------------------------------------------------------
    // Issue 1: Tests for newly extracted CpuBackend operations
    // -------------------------------------------------------------------

    #[test]
    fn test_slice_columns() {
        let b = backend();
        // 2x4 matrix, slice columns 1..3 → 2x2
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 2, 4);
        let dt = b.upload(&t);
        let sliced = b.slice_columns(&dt, 1, 3);
        let result = b.download(&sliced);
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 2);
        assert_eq!(result.data, vec![2.0, 3.0, 6.0, 7.0]);
    }

    #[test]
    fn test_slice_columns_full_width() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
        let dt = b.upload(&t);
        let sliced = b.slice_columns(&dt, 0, 3);
        let result = b.download(&sliced);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_slice_columns_single_column() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
        let dt = b.upload(&t);
        let sliced = b.slice_columns(&dt, 2, 3);
        let result = b.download(&sliced);
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 1);
        assert_eq!(result.data, vec![3.0, 6.0]);
    }

    #[test]
    fn test_scatter_columns() {
        let b = backend();
        // Destination: 2x4 zeros
        let mut dst = b.zeros(2, 4);
        // Source: 2x2 values
        let src_t = Tensor::from_slice(&[10.0, 20.0, 30.0, 40.0], 2, 2);
        let src = b.upload(&src_t);
        // Scatter at column offset 1
        b.scatter_columns(&mut dst, &src, 1);
        let result = b.download(&dst);
        assert_eq!(result.data, vec![0.0, 10.0, 20.0, 0.0, 0.0, 30.0, 40.0, 0.0]);
    }

    #[test]
    fn test_scatter_columns_at_start() {
        let b = backend();
        let mut dst = b.zeros(2, 3);
        let src_t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let src = b.upload(&src_t);
        b.scatter_columns(&mut dst, &src, 0);
        let result = b.download(&dst);
        assert_eq!(result.data, vec![1.0, 2.0, 0.0, 3.0, 4.0, 0.0]);
    }

    #[test]
    fn test_slice_scatter_roundtrip() {
        let b = backend();
        // Create 2x6, slice cols 2..5, scatter back into a fresh 2x6 at offset 2
        let t = Tensor::from_slice(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
              7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
            2, 6,
        );
        let dt = b.upload(&t);
        let sliced = b.slice_columns(&dt, 2, 5);
        let mut dst = b.zeros(2, 6);
        b.scatter_columns(&mut dst, &sliced, 2);
        let result = b.download(&dst);
        assert_eq!(
            result.data,
            vec![0.0, 0.0, 3.0, 4.0, 5.0, 0.0,
                 0.0, 0.0, 9.0, 10.0, 11.0, 0.0]
        );
    }

    #[test]
    fn test_apply_attention_mask_basic() {
        let b = backend();
        // 2x3 scores, mask=[1,0,1] → column 1 should become -10000
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
        let mut dt = b.upload(&t);
        let mask = b.upload_mask(&[1, 0, 1]);
        b.apply_attention_mask(&mut dt, &mask);
        let result = b.download(&dt);
        assert_eq!(result.data[0], 1.0);
        assert_eq!(result.data[1], -10000.0);
        assert_eq!(result.data[2], 3.0);
        assert_eq!(result.data[3], 4.0);
        assert_eq!(result.data[4], -10000.0);
        assert_eq!(result.data[5], 6.0);
    }

    #[test]
    fn test_apply_attention_mask_all_ones() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0], 1, 3);
        let mut dt = b.upload(&t);
        let mask = b.upload_mask(&[1, 1, 1]);
        b.apply_attention_mask(&mut dt, &mask);
        let result = b.download(&dt);
        assert_eq!(result.data, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_apply_attention_mask_all_zeros() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0], 1, 2);
        let mut dt = b.upload(&t);
        let mask = b.upload_mask(&[0, 0]);
        b.apply_attention_mask(&mut dt, &mask);
        let result = b.download(&dt);
        assert_eq!(result.data, vec![-10000.0, -10000.0]);
    }

    #[test]
    fn test_mean_pool_basic() {
        let b = backend();
        // 3 rows x 2 cols, mask=[1,1,0] → average of rows 0 and 1
        let t = Tensor::from_slice(&[2.0, 4.0, 6.0, 8.0, 100.0, 200.0], 3, 2);
        let dt = b.upload(&t);
        let mask = b.upload_mask(&[1, 1, 0]);
        let result = b.mean_pool(&dt, &mask);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 4.0).abs() < 1e-6); // (2+6)/2
        assert!((result[1] - 6.0).abs() < 1e-6); // (4+8)/2
    }

    #[test]
    fn test_mean_pool_single_token() {
        let b = backend();
        let t = Tensor::from_slice(&[3.0, 7.0, 100.0, 200.0], 2, 2);
        let dt = b.upload(&t);
        let mask = b.upload_mask(&[1, 0]);
        let result = b.mean_pool(&dt, &mask);
        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_mean_pool_all_masked() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let dt = b.upload(&t);
        let mask = b.upload_mask(&[0, 0]);
        let result = b.mean_pool(&dt, &mask);
        // No tokens contribute → result is zeros (sum=0, count=0 guard)
        assert_eq!(result, vec![0.0, 0.0]);
    }

    // -------------------------------------------------------------------
    // Issue 2: Backend round-trip tests — verify upload → op → download
    //          matches the direct Tensor method for every operation.
    // -------------------------------------------------------------------

    #[test]
    fn test_roundtrip_matmul() {
        let b = backend();
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
        let c = Tensor::from_slice(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2);
        let expected = a.matmul(&c);

        let da = b.upload(&a);
        let dc = b.upload(&c);
        let result = b.download(&b.matmul(&da, &dc));
        assert_eq!(result.data, expected.data);
    }

    #[test]
    fn test_roundtrip_matmul_transpose() {
        let b = backend();
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let c = Tensor::from_slice(&[5.0, 6.0, 7.0, 8.0], 2, 2);
        let expected = a.matmul_transpose(&c);

        let da = b.upload(&a);
        let dc = b.upload(&c);
        let result = b.download(&b.matmul_transpose(&da, &dc));
        assert_eq!(result.data, expected.data);
    }

    #[test]
    fn test_roundtrip_add_bias() {
        let b = backend();
        let mut t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let bias = [10.0, 20.0];
        let mut dt = b.upload(&t);
        let dbias = b.upload_1d(&bias);
        b.add_bias(&mut dt, &dbias);
        let result = b.download(&dt);

        t.add_bias(&bias);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_roundtrip_add_tensor() {
        let b = backend();
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let c = Tensor::from_slice(&[10.0, 20.0, 30.0, 40.0], 2, 2);
        let expected = a.add_tensor(&c);

        let da = b.upload(&a);
        let dc = b.upload(&c);
        let result = b.download(&b.add_tensor(&da, &dc));
        assert_eq!(result.data, expected.data);
    }

    #[test]
    fn test_roundtrip_gelu() {
        let b = backend();
        let t = Tensor::from_slice(&[-2.0, -1.0, 0.0, 1.0, 2.0, 5.0], 2, 3);
        let expected = t.gelu();

        let dt = b.upload(&t);
        let result = b.download(&b.gelu(&dt));
        for (e, r) in expected.data.iter().zip(result.data.iter()) {
            assert!((e - r).abs() < 1e-6, "gelu mismatch: expected {}, got {}", e, r);
        }
    }

    #[test]
    fn test_roundtrip_layer_norm() {
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 3.0, 2.0, 6.0], 2, 2);
        let w = vec![1.0, 1.0];
        let bias = vec![0.0, 0.0];
        let expected = t.layer_norm(&w, &bias, 1e-5);

        let dt = b.upload(&t);
        let dw = b.upload_1d(&w);
        let db = b.upload_1d(&bias);
        let result = b.download(&b.layer_norm(&dt, &dw, &db, 1e-5));
        for (e, r) in expected.data.iter().zip(result.data.iter()) {
            assert!((e - r).abs() < 1e-4, "layer_norm mismatch: expected {}, got {}", e, r);
        }
    }

    #[test]
    fn test_roundtrip_softmax_rows() {
        let b = backend();
        let mut t = Tensor::from_slice(&[1.0, 2.0, 3.0, 10.0, 20.0, 30.0], 2, 3);
        let mut dt = b.upload(&t);
        b.softmax_rows(&mut dt);
        let result = b.download(&dt);

        t.softmax_rows();
        for (e, r) in t.data.iter().zip(result.data.iter()) {
            assert!((e - r).abs() < 1e-6, "softmax mismatch: expected {}, got {}", e, r);
        }
    }

    #[test]
    fn test_roundtrip_scale() {
        let b = backend();
        let mut t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 2, 2);
        let mut dt = b.upload(&t);
        b.scale(&mut dt, 0.5);
        let result = b.download(&dt);

        t.scale(0.5);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_roundtrip_upload_download_identity() {
        let b = backend();
        let t = Tensor::from_slice(&[1.5, -2.5, 3.14, 0.0, f32::MAX, f32::MIN], 2, 3);
        let dt = b.upload(&t);
        let result = b.download(&dt);
        assert_eq!(result.rows, t.rows);
        assert_eq!(result.cols, t.cols);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_roundtrip_zeros() {
        let b = backend();
        let dt = b.zeros(3, 4);
        let result = b.download(&dt);
        assert_eq!(result.rows, 3);
        assert_eq!(result.cols, 4);
        assert!(result.data.iter().all(|&v| v == 0.0));
    }

    // -------------------------------------------------------------------
    // Batched operation tests — verify batched results match sequential
    // per-block calls to the non-batched equivalents.
    // -------------------------------------------------------------------

    #[test]
    fn test_batched_matmul_transpose_matches_individual() {
        let b = backend();
        let batch_size = 3;
        let seq_len = 4;
        let k = 5;

        // Build per-batch A and B matrices, then concatenate
        let mut a_data = Vec::new();
        let mut b_data = Vec::new();
        let mut expected_data = Vec::new();

        for batch in 0..batch_size {
            let a_block: Vec<f32> = (0..seq_len * k)
                .map(|i| (batch * 100 + i) as f32 * 0.1)
                .collect();
            let b_block: Vec<f32> = (0..seq_len * k)
                .map(|i| (batch * 200 + i) as f32 * 0.05)
                .collect();

            // Compute expected: A_block * B_block^T via non-batched matmul_transpose
            let a_tensor = Tensor::from_slice(&a_block, seq_len, k);
            let b_tensor = Tensor::from_slice(&b_block, seq_len, k);
            let da = b.upload(&a_tensor);
            let db_t = b.upload(&b_tensor);
            let result = b.download(&b.matmul_transpose(&da, &db_t));
            expected_data.extend_from_slice(&result.data);

            a_data.extend_from_slice(&a_block);
            b_data.extend_from_slice(&b_block);
        }

        // Now compute via batched method
        let a_full = Tensor::from_slice(&a_data, batch_size * seq_len, k);
        let b_full = Tensor::from_slice(&b_data, batch_size * seq_len, k);
        let da = b.upload(&a_full);
        let db_t = b.upload(&b_full);
        let batched_result = b.download(&b.batched_matmul_transpose(&da, &db_t, batch_size, seq_len));

        assert_eq!(batched_result.rows, batch_size * seq_len);
        assert_eq!(batched_result.cols, seq_len);
        for (i, (e, r)) in expected_data.iter().zip(batched_result.data.iter()).enumerate() {
            assert!(
                (e - r).abs() < 1e-4,
                "batched_matmul_transpose mismatch at index {}: expected {}, got {}",
                i, e, r
            );
        }
    }

    #[test]
    fn test_batched_matmul_matches_individual() {
        let b = backend();
        let batch_size = 3;
        let seq_len = 4;
        let k = 5;

        // batched_matmul: A: (batch*S, S), B: (batch*S, K) → C: (batch*S, K)
        let mut a_data = Vec::new();
        let mut b_data = Vec::new();
        let mut expected_data = Vec::new();

        for batch in 0..batch_size {
            let a_block: Vec<f32> = (0..seq_len * seq_len)
                .map(|i| (batch * 100 + i) as f32 * 0.01)
                .collect();
            let b_block: Vec<f32> = (0..seq_len * k)
                .map(|i| (batch * 200 + i) as f32 * 0.02)
                .collect();

            let a_tensor = Tensor::from_slice(&a_block, seq_len, seq_len);
            let b_tensor = Tensor::from_slice(&b_block, seq_len, k);
            let da = b.upload(&a_tensor);
            let db_t = b.upload(&b_tensor);
            let result = b.download(&b.matmul(&da, &db_t));
            expected_data.extend_from_slice(&result.data);

            a_data.extend_from_slice(&a_block);
            b_data.extend_from_slice(&b_block);
        }

        let a_full = Tensor::from_slice(&a_data, batch_size * seq_len, seq_len);
        let b_full = Tensor::from_slice(&b_data, batch_size * seq_len, k);
        let da = b.upload(&a_full);
        let db_t = b.upload(&b_full);
        let batched_result = b.download(&b.batched_matmul(&da, &db_t, batch_size, seq_len));

        assert_eq!(batched_result.rows, batch_size * seq_len);
        assert_eq!(batched_result.cols, k);
        for (i, (e, r)) in expected_data.iter().zip(batched_result.data.iter()).enumerate() {
            assert!(
                (e - r).abs() < 1e-4,
                "batched_matmul mismatch at index {}: expected {}, got {}",
                i, e, r
            );
        }
    }

    #[test]
    fn test_batched_matmul_transpose_single_batch() {
        // Single batch should produce identical result to plain matmul_transpose
        let b = backend();
        let seq_len = 5;
        let k = 3;

        let a_data: Vec<f32> = (0..seq_len * k).map(|i| i as f32 * 0.3).collect();
        let b_data: Vec<f32> = (0..seq_len * k).map(|i| (i + 7) as f32 * 0.2).collect();

        let a = Tensor::from_slice(&a_data, seq_len, k);
        let bt = Tensor::from_slice(&b_data, seq_len, k);
        let da = b.upload(&a);
        let db = b.upload(&bt);

        let individual = b.download(&b.matmul_transpose(&da, &db));
        let batched = b.download(&b.batched_matmul_transpose(&da, &db, 1, seq_len));

        assert_eq!(individual.rows, batched.rows);
        assert_eq!(individual.cols, batched.cols);
        assert_eq!(individual.data, batched.data);
    }

    #[test]
    fn test_batched_matmul_single_batch() {
        // Single batch should produce identical result to plain matmul
        let b = backend();
        let seq_len = 4;
        let k = 6;

        let a_data: Vec<f32> = (0..seq_len * seq_len).map(|i| i as f32 * 0.1).collect();
        let b_data: Vec<f32> = (0..seq_len * k).map(|i| (i + 3) as f32 * 0.2).collect();

        let a = Tensor::from_slice(&a_data, seq_len, seq_len);
        let bt = Tensor::from_slice(&b_data, seq_len, k);
        let da = b.upload(&a);
        let db = b.upload(&bt);

        let individual = b.download(&b.matmul(&da, &db));
        let batched = b.download(&b.batched_matmul(&da, &db, 1, seq_len));

        assert_eq!(individual.rows, batched.rows);
        assert_eq!(individual.cols, batched.cols);
        assert_eq!(individual.data, batched.data);
    }

    #[test]
    fn test_batched_matmul_transpose_no_cross_batch_leakage() {
        // Verify that batch 0's data doesn't affect batch 1's result.
        // Set batch 0 to all zeros, batch 1 to known values.
        let b = backend();
        let seq_len = 3;
        let k = 2;

        // Batch 0: all zeros. Batch 1: known values.
        let mut a_data = vec![0.0f32; seq_len * k]; // batch 0
        let a1: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // batch 1
        a_data.extend_from_slice(&a1);

        let mut b_data = vec![0.0f32; seq_len * k]; // batch 0
        let b1: Vec<f32> = vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]; // batch 1
        b_data.extend_from_slice(&b1);

        let a = Tensor::from_slice(&a_data, 2 * seq_len, k);
        let bt = Tensor::from_slice(&b_data, 2 * seq_len, k);
        let da = b.upload(&a);
        let db = b.upload(&bt);
        let result = b.download(&b.batched_matmul_transpose(&da, &db, 2, seq_len));

        // Batch 0 block: all zeros × all zeros = all zeros
        for i in 0..seq_len * seq_len {
            assert_eq!(result.data[i], 0.0, "batch 0 should be all zeros at index {}", i);
        }

        // Batch 1 block: verify against individual matmul_transpose
        let a1_t = Tensor::from_slice(&a1, seq_len, k);
        let b1_t = Tensor::from_slice(&b1, seq_len, k);
        let expected = b.download(&b.matmul_transpose(&b.upload(&a1_t), &b.upload(&b1_t)));
        let batch1_start = seq_len * seq_len;
        for i in 0..seq_len * seq_len {
            assert!(
                (result.data[batch1_start + i] - expected.data[i]).abs() < 1e-6,
                "batch 1 mismatch at index {}: expected {}, got {}",
                i, expected.data[i], result.data[batch1_start + i]
            );
        }
    }

    #[test]
    fn test_batched_attention_mask_matches_individual() {
        let b = backend();
        let batch_size = 3;
        let seq_len = 4;

        // Build masks with different active token counts per batch
        // batch 0: all active [1,1,1,1]
        // batch 1: 2 active [1,1,0,0]
        // batch 2: 1 active [1,0,0,0]
        let mask_data = vec![
            1, 1, 1, 1, // batch 0
            1, 1, 0, 0, // batch 1
            1, 0, 0, 0, // batch 2
        ];

        // Create score matrices
        let scores_data: Vec<f32> = (0..batch_size * seq_len * seq_len)
            .map(|i| i as f32 * 0.5)
            .collect();

        // Compute expected: apply per-batch mask individually
        let mut expected = scores_data.clone();
        for batch in 0..batch_size {
            let batch_mask = &mask_data[batch * seq_len..(batch + 1) * seq_len];
            for row in 0..seq_len {
                for col in 0..seq_len {
                    if batch_mask[col] == 0 {
                        expected[(batch * seq_len + row) * seq_len + col] = -10000.0;
                    }
                }
            }
        }

        // Compute via batched method
        let scores_tensor = Tensor::from_slice(&scores_data, batch_size * seq_len, seq_len);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.batched_attention_mask(&mut dt, &mask, batch_size, seq_len);
        let result = b.download(&dt);

        assert_eq!(result.data, expected);
    }

    #[test]
    fn test_batched_attention_mask_all_active() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let mask_data = vec![1u32; batch_size * seq_len];
        let scores_data: Vec<f32> = (0..batch_size * seq_len * seq_len)
            .map(|i| i as f32)
            .collect();

        let scores_tensor = Tensor::from_slice(&scores_data, batch_size * seq_len, seq_len);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.batched_attention_mask(&mut dt, &mask, batch_size, seq_len);
        let result = b.download(&dt);

        // No positions masked → scores unchanged
        assert_eq!(result.data, scores_data);
    }

    #[test]
    fn test_batched_attention_mask_all_masked() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let mask_data = vec![0u32; batch_size * seq_len];
        let scores_data: Vec<f32> = (0..batch_size * seq_len * seq_len)
            .map(|i| i as f32)
            .collect();

        let scores_tensor = Tensor::from_slice(&scores_data, batch_size * seq_len, seq_len);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.batched_attention_mask(&mut dt, &mask, batch_size, seq_len);
        let result = b.download(&dt);

        // All positions masked → all -10000
        assert!(result.data.iter().all(|&v| v == -10000.0));
    }

    #[test]
    fn test_batched_mean_pool_matches_individual() {
        let b = backend();
        let batch_size = 3;
        let seq_len = 4;
        let d = 5;

        // Masks with different active counts per batch
        let mask_data = vec![
            1, 1, 1, 1, // batch 0: all active
            1, 1, 0, 0, // batch 1: 2 active
            1, 0, 0, 0, // batch 2: 1 active
        ];

        let hidden_data: Vec<f32> = (0..batch_size * seq_len * d)
            .map(|i| i as f32 * 0.1)
            .collect();
        let hidden = Tensor::from_slice(&hidden_data, batch_size * seq_len, d);
        let dh = b.upload(&hidden);
        let mask = b.upload_mask(&mask_data);

        // Compute via batched method
        let batched_results = b.batched_mean_pool(&dh, &mask, batch_size, seq_len);
        assert_eq!(batched_results.len(), batch_size);

        // Verify each batch against individual mean_pool
        for batch in 0..batch_size {
            let start_row = batch * seq_len;
            let block_data: Vec<f32> = (0..seq_len * d)
                .map(|i| hidden_data[start_row * d + i])
                .collect();
            let block_tensor = Tensor::from_slice(&block_data, seq_len, d);
            let block_mask = b.upload_mask(&mask_data[batch * seq_len..(batch + 1) * seq_len]);
            let db = b.upload(&block_tensor);
            let individual = b.mean_pool(&db, &block_mask);

            assert_eq!(batched_results[batch].len(), d);
            for (j, (e, r)) in individual.iter().zip(batched_results[batch].iter()).enumerate() {
                assert!(
                    (e - r).abs() < 1e-5,
                    "batch {} dim {} mismatch: expected {}, got {}",
                    batch, j, e, r
                );
            }
        }
    }

    #[test]
    fn test_batched_mean_pool_single_batch() {
        // Single batch should match plain mean_pool exactly
        let b = backend();
        let seq_len = 5;
        let d = 3;
        let mask_data = vec![1, 1, 0, 1, 0];
        let data: Vec<f32> = (0..seq_len * d).map(|i| i as f32 * 0.7).collect();

        let tensor = Tensor::from_slice(&data, seq_len, d);
        let dt = b.upload(&tensor);
        let mask = b.upload_mask(&mask_data);

        let individual = b.mean_pool(&dt, &mask);
        let batched = b.batched_mean_pool(&dt, &mask, 1, seq_len);

        assert_eq!(batched.len(), 1);
        assert_eq!(individual, batched[0]);
    }

    #[test]
    fn test_batched_mean_pool_all_masked() {
        // All masked → should return zero vectors
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let d = 4;
        let mask_data = vec![0u32; batch_size * seq_len];
        let data: Vec<f32> = (0..batch_size * seq_len * d)
            .map(|i| (i + 1) as f32)
            .collect();

        let tensor = Tensor::from_slice(&data, batch_size * seq_len, d);
        let dt = b.upload(&tensor);
        let mask = b.upload_mask(&mask_data);
        let results = b.batched_mean_pool(&dt, &mask, batch_size, seq_len);

        for (batch, result) in results.iter().enumerate() {
            assert!(
                result.iter().all(|&v| v == 0.0),
                "batch {} should be all zeros when fully masked",
                batch
            );
        }
    }

    #[test]
    fn test_batched_mean_pool_single_active_per_batch() {
        // Only 1 active token per batch → result should be that token's values exactly
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let d = 2;
        // batch 0: only row 0 active, batch 1: only row 2 (index 5) active
        let mask_data = vec![1, 0, 0, 0, 0, 1];
        let data = vec![
            10.0, 20.0, // batch 0 row 0 (active)
            30.0, 40.0, // batch 0 row 1
            50.0, 60.0, // batch 0 row 2
            70.0, 80.0, // batch 1 row 0
            90.0, 100.0, // batch 1 row 1
            110.0, 120.0, // batch 1 row 2 (active)
        ];

        let tensor = Tensor::from_slice(&data, batch_size * seq_len, d);
        let dt = b.upload(&tensor);
        let mask = b.upload_mask(&mask_data);
        let results = b.batched_mean_pool(&dt, &mask, batch_size, seq_len);

        // Batch 0: mean of just [10, 20] = [10, 20]
        assert_eq!(results[0], vec![10.0, 20.0]);
        // Batch 1: mean of just [110, 120] = [110, 120]
        assert_eq!(results[1], vec![110.0, 120.0]);
    }

    // -------------------------------------------------------------------
    // transpose_heads / untranspose_heads tests
    // -------------------------------------------------------------------

    #[test]
    fn test_transpose_heads_basic() {
        let b = backend();
        // batch=1, seq_len=2, num_heads=2, head_dim=3
        // Input (2, 6): row 0 = [h0d0, h0d1, h0d2, h1d0, h1d1, h1d2]
        //               row 1 = [h0d3, h0d4, h0d5, h1d3, h1d4, h1d5]
        let t = Tensor::from_slice(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
              7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
            2, 6,
        );
        let dt = b.upload(&t);
        let result = b.download(&b.transpose_heads(&dt, 1, 2, 2, 3));
        // Output (4, 3): B*H*S=1*2*2=4 rows, D=3 cols
        // head 0: token 0 = [1,2,3], token 1 = [7,8,9]
        // head 1: token 0 = [4,5,6], token 1 = [10,11,12]
        assert_eq!(result.rows, 4);
        assert_eq!(result.cols, 3);
        assert_eq!(
            result.data,
            vec![1.0, 2.0, 3.0,   // head 0, token 0
                 7.0, 8.0, 9.0,   // head 0, token 1
                 4.0, 5.0, 6.0,   // head 1, token 0
                 10.0, 11.0, 12.0] // head 1, token 1
        );
    }

    #[test]
    fn test_untranspose_heads_basic() {
        let b = backend();
        // Input (4, 3): B*H*S=4, D=3 (from transpose_heads output)
        let t = Tensor::from_slice(
            &[1.0, 2.0, 3.0,     // head 0, token 0
              7.0, 8.0, 9.0,     // head 0, token 1
              4.0, 5.0, 6.0,     // head 1, token 0
              10.0, 11.0, 12.0], // head 1, token 1
            4, 3,
        );
        let dt = b.upload(&t);
        let result = b.download(&b.untranspose_heads(&dt, 1, 2, 2, 3));
        // Output (2, 6): restored original layout
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 6);
        assert_eq!(
            result.data,
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
                 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
        );
    }

    #[test]
    fn test_transpose_untranspose_roundtrip() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let num_heads = 4;
        let head_dim = 5;
        let total = batch_size * seq_len * num_heads * head_dim;

        let data: Vec<f32> = (0..total).map(|i| i as f32 * 0.1).collect();
        let t = Tensor::from_slice(&data, batch_size * seq_len, num_heads * head_dim);
        let dt = b.upload(&t);

        let transposed = b.transpose_heads(&dt, batch_size, seq_len, num_heads, head_dim);
        assert_eq!(transposed.rows, batch_size * num_heads * seq_len);
        assert_eq!(transposed.cols, head_dim);

        let restored = b.download(&b.untranspose_heads(
            &transposed, batch_size, seq_len, num_heads, head_dim,
        ));
        assert_eq!(restored.rows, batch_size * seq_len);
        assert_eq!(restored.cols, num_heads * head_dim);
        assert_eq!(restored.data, data);
    }

    #[test]
    fn test_transpose_heads_single_token() {
        // Edge case: seq_len=1
        let b = backend();
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], 1, 4);
        let dt = b.upload(&t);
        let result = b.download(&b.transpose_heads(&dt, 1, 1, 2, 2));
        // (1, 4) -> (2, 2): head 0 = [1,2], head 1 = [3,4]
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 2);
        assert_eq!(result.data, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_transpose_heads_matches_minilm_shape() {
        // Verify with actual MiniLM dimensions
        let b = backend();
        let batch_size = 2;
        let seq_len = 8;
        let num_heads = 12;
        let head_dim = 32;

        let total = batch_size * seq_len * num_heads * head_dim;
        let data: Vec<f32> = (0..total).map(|i| (i % 100) as f32 * 0.01).collect();
        let t = Tensor::from_slice(&data, batch_size * seq_len, num_heads * head_dim);
        let dt = b.upload(&t);

        let transposed = b.transpose_heads(&dt, batch_size, seq_len, num_heads, head_dim);
        assert_eq!(transposed.rows, batch_size * num_heads * seq_len); // 2*12*8=192
        assert_eq!(transposed.cols, head_dim); // 32

        // Roundtrip
        let restored = b.download(&b.untranspose_heads(
            &transposed, batch_size, seq_len, num_heads, head_dim,
        ));
        assert_eq!(restored.data, data);
    }

    // -------------------------------------------------------------------
    // multi_head_batched_attention_mask tests
    // -------------------------------------------------------------------

    #[test]
    fn test_multi_head_mask_basic() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let num_heads = 2;

        // Mask: batch 0 = [1,1,0], batch 1 = [1,0,0]
        let mask_data = vec![1, 1, 0, 1, 0, 0];

        // Scores: (B*H*S, S) = (12, 3)
        let scores_data: Vec<f32> = (0..12 * 3).map(|i| i as f32).collect();
        let scores_tensor = Tensor::from_slice(&scores_data, 12, 3);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);

        b.multi_head_batched_attention_mask(&mut dt, &mask, batch_size, seq_len, num_heads);
        let result = b.download(&dt);

        // group_size = H*S = 2*3 = 6
        // Rows 0-5: group = 0 (batch 0), mask = [1,1,0]
        // Rows 6-11: group = 1 (batch 1), mask = [1,0,0]
        for row in 0..6 {
            assert_eq!(result.data[row * 3 + 0], scores_data[row * 3 + 0]); // mask=1
            assert_eq!(result.data[row * 3 + 1], scores_data[row * 3 + 1]); // mask=1
            assert_eq!(result.data[row * 3 + 2], -10000.0);                 // mask=0
        }
        for row in 6..12 {
            assert_eq!(result.data[row * 3 + 0], scores_data[row * 3 + 0]); // mask=1
            assert_eq!(result.data[row * 3 + 1], -10000.0);                 // mask=0
            assert_eq!(result.data[row * 3 + 2], -10000.0);                 // mask=0
        }
    }

    #[test]
    fn test_multi_head_mask_single_batch_matches_apply_attention_mask() {
        // multi_head_batched_attention_mask with batch_size=1 should produce
        // the same result as apply_attention_mask applied to each head's scores.
        let b = backend();
        let seq_len = 4;
        let num_heads = 3;

        let mask_data = vec![1u32, 1, 0, 1];
        let scores_data: Vec<f32> = (0..(num_heads * seq_len * seq_len))
            .map(|i| i as f32 * 0.5)
            .collect();

        // Method 1: multi_head_batched_attention_mask
        let scores_tensor = Tensor::from_slice(
            &scores_data,
            num_heads * seq_len,
            seq_len,
        );
        let mut dt1 = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.multi_head_batched_attention_mask(&mut dt1, &mask, 1, seq_len, num_heads);
        let result1 = b.download(&dt1);

        // Method 2: apply_attention_mask per head
        let mut expected = scores_data.clone();
        for row in 0..(num_heads * seq_len) {
            for col in 0..seq_len {
                if mask_data[col] == 0 {
                    expected[row * seq_len + col] = -10000.0;
                }
            }
        }

        assert_eq!(result1.data, expected);
    }

    #[test]
    fn test_multi_head_mask_all_active() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let num_heads = 2;
        let total_rows = batch_size * num_heads * seq_len;

        let mask_data = vec![1u32; batch_size * seq_len];
        let scores_data: Vec<f32> = (0..total_rows * seq_len)
            .map(|i| i as f32)
            .collect();

        let scores_tensor = Tensor::from_slice(&scores_data, total_rows, seq_len);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.multi_head_batched_attention_mask(&mut dt, &mask, batch_size, seq_len, num_heads);
        let result = b.download(&dt);

        // All active → scores unchanged
        assert_eq!(result.data, scores_data);
    }

    #[test]
    fn test_multi_head_mask_all_masked() {
        let b = backend();
        let batch_size = 2;
        let seq_len = 3;
        let num_heads = 2;
        let total_rows = batch_size * num_heads * seq_len;

        let mask_data = vec![0u32; batch_size * seq_len];
        let scores_data: Vec<f32> = (0..total_rows * seq_len)
            .map(|i| i as f32)
            .collect();

        let scores_tensor = Tensor::from_slice(&scores_data, total_rows, seq_len);
        let mut dt = b.upload(&scores_tensor);
        let mask = b.upload_mask(&mask_data);
        b.multi_head_batched_attention_mask(&mut dt, &mask, batch_size, seq_len, num_heads);
        let result = b.download(&dt);

        assert!(result.data.iter().all(|&v| v == -10000.0));
    }
}
