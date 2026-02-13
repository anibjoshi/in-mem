//! Compute backend trait for GPU-accelerated tensor operations.
//!
//! Provides a `ComputeBackend` trait that abstracts tensor operations across
//! CPU, CUDA, and Metal. Backend selection happens once at model load time.

use std::any::Any;
use std::sync::Arc;

use super::tensor::Tensor;

/// A tensor that lives on a compute device (CPU, CUDA, or Metal).
///
/// The inner representation is backend-specific and type-erased.
pub struct DeviceTensor {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
    /// Backend-specific storage.
    pub(crate) inner: Box<dyn Any + Send + Sync>,
}

/// Trait for compute backends that execute tensor operations.
///
/// Implementations dispatch operations to CPU, CUDA, or Metal.
pub trait ComputeBackend: Send + Sync {
    /// Upload a CPU tensor to the device.
    fn upload(&self, t: &Tensor) -> DeviceTensor;

    /// Upload a 1D slice to the device as a single-row tensor.
    fn upload_1d(&self, v: &[f32]) -> DeviceTensor;

    /// Download a device tensor back to CPU.
    fn download(&self, dt: &DeviceTensor) -> Tensor;

    /// Matrix multiply: (M,K) x (K,N) -> (M,N).
    fn matmul(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor;

    /// Matrix multiply with transpose: (M,K) x (N,K)^T -> (M,N).
    fn matmul_transpose(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor;

    /// Broadcast row-add: add a 1-row bias to each row of t.
    fn add_bias(&self, t: &mut DeviceTensor, bias: &DeviceTensor);

    /// Element-wise add.
    fn add_tensor(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor;

    /// Fast GELU activation.
    fn gelu(&self, t: &DeviceTensor) -> DeviceTensor;

    /// Layer normalization per row.
    fn layer_norm(
        &self,
        t: &DeviceTensor,
        w: &DeviceTensor,
        b: &DeviceTensor,
        eps: f32,
    ) -> DeviceTensor;

    /// Per-row softmax in place.
    fn softmax_rows(&self, t: &mut DeviceTensor);

    /// Scalar multiply in place.
    fn scale(&self, t: &mut DeviceTensor, factor: f32);

    /// Extract a contiguous column slice from each row.
    fn slice_columns(&self, t: &DeviceTensor, start: usize, end: usize) -> DeviceTensor;

    /// Write src columns into dst starting at col_offset.
    fn scatter_columns(&self, dst: &mut DeviceTensor, src: &DeviceTensor, col_offset: usize);

    /// Create a zero tensor on the device.
    fn zeros(&self, rows: usize, cols: usize) -> DeviceTensor;

    /// Upload a u32 mask to the device for reuse across multiple calls.
    fn upload_mask(&self, mask: &[u32]) -> DeviceTensor;

    /// Apply attention mask: set padding positions to -10000.
    fn apply_attention_mask(&self, scores: &mut DeviceTensor, mask: &DeviceTensor);

    /// Mean pooling with attention mask, returns host vector.
    fn mean_pool(&self, hidden: &DeviceTensor, mask: &DeviceTensor) -> Vec<f32>;

    /// Block-diagonal matmul_transpose: for each batch b,
    /// C[b] = A[b] * B[b]^T. A,B: (batch*S, K), C: (batch*S, S).
    fn batched_matmul_transpose(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor;

    /// Block-diagonal matmul: for each batch b,
    /// C[b] = A[b] * B[b]. A: (batch*S, S), B: (batch*S, K), C: (batch*S, K).
    fn batched_matmul(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor;

    /// Batched attention mask: for row r, batch = r / seq_len,
    /// if mask[batch * seq_len + col] == 0 then scores[r][col] = -10000.
    fn batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    );

    /// Batched mean pool: one pooled vector per sequence.
    /// hidden: (batch*S, D), mask: (batch*S,). Returns batch_size vectors.
    fn batched_mean_pool(
        &self,
        hidden: &DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> Vec<Vec<f32>>;

    /// Rearrange (B*S, H*D) -> (B*H*S, D) so heads become batches.
    ///
    /// Default implementation uses slice_columns per head (functional but slow).
    fn transpose_heads(
        &self,
        t: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
        head_dim: usize,
    ) -> DeviceTensor {
        let total_rows = batch_size * seq_len;
        debug_assert_eq!(t.rows, total_rows);
        debug_assert_eq!(t.cols, num_heads * head_dim);

        // Download, rearrange on CPU, re-upload.
        let src = self.download(t);
        let mut dst = vec![0.0f32; batch_size * num_heads * seq_len * head_dim];
        for b in 0..batch_size {
            for h in 0..num_heads {
                for s in 0..seq_len {
                    let src_offset = (b * seq_len + s) * (num_heads * head_dim) + h * head_dim;
                    let dst_offset = (b * num_heads * seq_len + h * seq_len + s) * head_dim;
                    dst[dst_offset..dst_offset + head_dim]
                        .copy_from_slice(&src.data[src_offset..src_offset + head_dim]);
                }
            }
        }
        self.upload(&Tensor::from_slice(
            &dst,
            batch_size * num_heads * seq_len,
            head_dim,
        ))
    }

    /// Rearrange (B*H*S, D) -> (B*S, H*D), inverse of transpose_heads.
    ///
    /// Default implementation uses download/rearrange/upload (functional but slow).
    fn untranspose_heads(
        &self,
        t: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
        head_dim: usize,
    ) -> DeviceTensor {
        debug_assert_eq!(t.rows, batch_size * num_heads * seq_len);
        debug_assert_eq!(t.cols, head_dim);

        let src = self.download(t);
        let mut dst = vec![0.0f32; batch_size * seq_len * num_heads * head_dim];
        for b in 0..batch_size {
            for h in 0..num_heads {
                for s in 0..seq_len {
                    let src_offset = (b * num_heads * seq_len + h * seq_len + s) * head_dim;
                    let dst_offset = (b * seq_len + s) * (num_heads * head_dim) + h * head_dim;
                    dst[dst_offset..dst_offset + head_dim]
                        .copy_from_slice(&src.data[src_offset..src_offset + head_dim]);
                }
            }
        }
        self.upload(&Tensor::from_slice(
            &dst,
            batch_size * seq_len,
            num_heads * head_dim,
        ))
    }

    /// Multi-head batched attention mask for (B*H*S, S) scores.
    ///
    /// Every H consecutive batches share one mask row. For row r:
    /// group = r / (H * S), mask_idx = group * S + col.
    ///
    /// Default implementation downloads scores and mask, applies masking on
    /// CPU, re-uploads. Works for GPU backends where `download` interprets
    /// u32 mask bytes as f32 (u32(0) maps to f32(0.0)). CpuBackend overrides
    /// this with a direct `Vec<u32>` implementation.
    fn multi_head_batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        _batch_size: usize,
        seq_len: usize,
        num_heads: usize,
    ) {
        let mut s = self.download(scores);
        let m = self.download(mask);
        let group_size = num_heads * seq_len;
        for r in 0..s.rows {
            let group = r / group_size;
            for c in 0..seq_len {
                let mask_idx = group * seq_len + c;
                // On GPU backends: u32(0) downloads as f32(0.0)
                if m.data[mask_idx] == 0.0 {
                    s.data[r * s.cols + c] = -10000.0;
                }
            }
        }
        *scores = self.upload(&s);
    }

    /// Backend name for logging.
    fn name(&self) -> &'static str;
}

/// Select the best available compute backend.
///
/// Tries CUDA first, then Metal, falls back to CPU.
pub fn select_backend() -> Arc<dyn ComputeBackend> {
    #[cfg(feature = "embed-cuda")]
    {
        match super::cuda::CudaBackend::try_new() {
            Ok(backend) => {
                tracing::info!(target: "strata::embed", "Using CUDA compute backend");
                return Arc::new(backend);
            }
            Err(e) => {
                tracing::info!(target: "strata::embed", error = %e, "CUDA not available, trying next backend");
            }
        }
    }

    #[cfg(all(feature = "embed-metal", target_os = "macos"))]
    {
        match super::metal::MetalBackend::try_new() {
            Ok(backend) => {
                tracing::info!(target: "strata::embed", "Using Metal compute backend");
                return Arc::new(backend);
            }
            Err(e) => {
                tracing::info!(target: "strata::embed", error = %e, "Metal not available, falling back to CPU");
            }
        }
    }

    tracing::info!(target: "strata::embed", "Using CPU compute backend");
    Arc::new(super::cpu_backend::CpuBackend)
}
