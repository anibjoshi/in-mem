//! CUDA compute backend for GPU-accelerated tensor operations.
//!
//! Loads the CUDA driver at runtime (no link-time dependency) and dispatches
//! all tensor operations to GPU kernels written in PTX. Falls back gracefully
//! if CUDA is not available — `CudaBackend::try_new()` returns `Err` and the
//! backend selector moves on to the next option.

use std::os::raw::c_void;
use std::sync::Arc;

use super::backend::{ComputeBackend, DeviceTensor};
use super::tensor::Tensor;
use ffi::{CUdeviceptr, CUfunction, CUmodule, CUstream, CublasApi, CudaApi};

pub mod ffi;
pub mod kernels;

// ---------------------------------------------------------------------------
// CudaBuffer — RAII wrapper for a device allocation
// ---------------------------------------------------------------------------

/// A buffer allocated on the CUDA device.
///
/// Automatically freed when dropped. Prefers `cuMemFreeAsync` (stream-ordered,
/// <1us) when available, falls back to synchronous `cuMemFree`.
struct CudaBuffer {
    ptr: CUdeviceptr,
    #[allow(dead_code)]
    len: usize, // in bytes — retained for debugging / future introspection
    api: Arc<CudaApi>,
    stream: CUstream,
}

impl Drop for CudaBuffer {
    fn drop(&mut self) {
        if self.ptr != 0 {
            // Try async free first (stream-ordered, fast). Fall back to sync
            // free if the stream has been destroyed or async API is unavailable.
            if self.api.mem_free_async(self.ptr, self.stream).is_err() {
                if let Err(e) = self.api.mem_free(self.ptr) {
                    tracing::warn!(target: "strata::embed", error = %e, "CUDA: failed to free device memory");
                }
            }
        }
    }
}

// SAFETY: CudaBuffer holds a u64 device pointer and an Arc to thread-safe API.
// The device pointer is just an integer handle; it does not reference host memory.
unsafe impl Send for CudaBuffer {}
unsafe impl Sync for CudaBuffer {}

// ---------------------------------------------------------------------------
// CudaBackend
// ---------------------------------------------------------------------------

/// CUDA compute backend.
///
/// Manages a CUDA context, stream, loaded PTX module, and pre-resolved kernel
/// function handles. All tensor operations are dispatched as asynchronous kernel
/// launches on the backend's stream, with synchronization at download boundaries.
pub struct CudaBackend {
    api: Arc<CudaApi>,
    stream: CUstream,
    module: CUmodule,
    cublas: Option<CublasApi>,

    // Pre-loaded kernel function handles
    fn_gemm: CUfunction,
    fn_gemm_transpose: CUfunction,
    fn_gelu: CUfunction,
    fn_add_tensor: CUfunction,
    fn_add_bias: CUfunction,
    fn_scale: CUfunction,
    fn_layer_norm: CUfunction,
    fn_softmax_rows: CUfunction,
    fn_slice_columns: CUfunction,
    fn_scatter_columns: CUfunction,
    fn_attention_mask: CUfunction,
    fn_mean_pool: CUfunction,
    fn_batched_gemm_transpose: CUfunction,
    fn_batched_gemm: CUfunction,
    fn_batched_attention_mask: CUfunction,
    fn_batched_mean_pool: CUfunction,
    fn_transpose_heads: CUfunction,
    fn_untranspose_heads: CUfunction,
    fn_multi_head_batched_attention_mask: CUfunction,
}

// SAFETY: All CUDA function handles are process-global, and the Driver API is
// documented as thread-safe. The stream is used behind &self which Rust's
// borrow checker already serialises for &mut operations.
unsafe impl Send for CudaBackend {}
unsafe impl Sync for CudaBackend {}

impl CudaBackend {
    /// Attempt to create a new CUDA backend.
    ///
    /// This will:
    /// 1. Load the CUDA driver library and initialise it.
    /// 2. Create a context on device 0.
    /// 3. Create a compute stream.
    /// 4. Load the PTX module containing all kernels.
    /// 5. Resolve every kernel function handle.
    ///
    /// Returns `Err` if any step fails (no CUDA driver, no GPU, PTX load error, etc.).
    pub fn try_new() -> Result<Self, String> {
        let api = Arc::new(CudaApi::load()?);

        let stream = api.stream_create()?;

        // Try to load cuBLAS for accelerated GEMM. Falls back to PTX if unavailable.
        let cublas = match CublasApi::load(stream) {
            Ok(api) => {
                tracing::info!(target: "strata::embed", "cuBLAS loaded for accelerated GEMM");
                Some(api)
            }
            Err(e) => {
                tracing::info!(target: "strata::embed", error = %e, "cuBLAS not available, using PTX GEMM kernels");
                None
            }
        };

        // Load the PTX module. cuModuleLoadData expects a null-terminated string.
        let ptx = kernels::PTX_MODULE;
        let module = api.module_load_data(ptx.as_ptr() as *const c_void)?;

        // Resolve all kernel functions.
        macro_rules! get_fn {
            ($name:expr) => {{
                let cname = concat!($name, "\0");
                let cstr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(cname.as_bytes()) };
                api.module_get_function(module, cstr)?
            }};
        }

        let fn_gemm = get_fn!("gemm");
        let fn_gemm_transpose = get_fn!("gemm_transpose");
        let fn_gelu = get_fn!("gelu");
        let fn_add_tensor = get_fn!("add_tensor");
        let fn_add_bias = get_fn!("add_bias");
        let fn_scale = get_fn!("scale");
        let fn_layer_norm = get_fn!("layer_norm");
        let fn_softmax_rows = get_fn!("softmax_rows");
        let fn_slice_columns = get_fn!("slice_columns");
        let fn_scatter_columns = get_fn!("scatter_columns");
        let fn_attention_mask = get_fn!("attention_mask");
        let fn_mean_pool = get_fn!("mean_pool");
        let fn_batched_gemm_transpose = get_fn!("batched_gemm_transpose");
        let fn_batched_gemm = get_fn!("batched_gemm");
        let fn_batched_attention_mask = get_fn!("batched_attention_mask");
        let fn_batched_mean_pool = get_fn!("batched_mean_pool");
        let fn_transpose_heads = get_fn!("transpose_heads");
        let fn_untranspose_heads = get_fn!("untranspose_heads");
        let fn_multi_head_batched_attention_mask = get_fn!("multi_head_batched_attention_mask");

        Ok(Self {
            api,
            stream,
            module,
            cublas,
            fn_gemm,
            fn_gemm_transpose,
            fn_gelu,
            fn_add_tensor,
            fn_add_bias,
            fn_scale,
            fn_layer_norm,
            fn_softmax_rows,
            fn_slice_columns,
            fn_scatter_columns,
            fn_attention_mask,
            fn_mean_pool,
            fn_batched_gemm_transpose,
            fn_batched_gemm,
            fn_batched_attention_mask,
            fn_batched_mean_pool,
            fn_transpose_heads,
            fn_untranspose_heads,
            fn_multi_head_batched_attention_mask,
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Allocate device memory and copy host f32 data to it.
    fn upload_f32(&self, data: &[f32]) -> Result<CudaBuffer, String> {
        let bytesize = data.len() * std::mem::size_of::<f32>();
        let ptr = self.api.mem_alloc_async(bytesize, self.stream)?;
        self.api
            .memcpy_h_to_d(ptr, data.as_ptr() as *const c_void, bytesize)?;
        Ok(CudaBuffer {
            ptr,
            len: bytesize,
            api: Arc::clone(&self.api),
            stream: self.stream,
        })
    }

    /// Allocate zeroed device memory for `n` f32 elements.
    fn alloc_zeros_f32(&self, n: usize) -> Result<CudaBuffer, String> {
        let bytesize = n * std::mem::size_of::<f32>();
        let ptr = self.api.mem_alloc_async(bytesize, self.stream)?;
        // cuMemsetD32Async sets each 32-bit word on the compute stream;
        // 0u32 corresponds to 0.0f32.
        self.api.memset_d32_async(ptr, 0, n, self.stream)?;
        Ok(CudaBuffer {
            ptr,
            len: bytesize,
            api: Arc::clone(&self.api),
            stream: self.stream,
        })
    }

    /// Upload u32 data to the device.
    fn upload_u32(&self, data: &[u32]) -> Result<CudaBuffer, String> {
        let bytesize = data.len() * std::mem::size_of::<u32>();
        let ptr = self.api.mem_alloc_async(bytesize, self.stream)?;
        self.api
            .memcpy_h_to_d(ptr, data.as_ptr() as *const c_void, bytesize)?;
        Ok(CudaBuffer {
            ptr,
            len: bytesize,
            api: Arc::clone(&self.api),
            stream: self.stream,
        })
    }

    /// Download `n` f32 elements from device to host.
    fn download_f32(&self, ptr: CUdeviceptr, n: usize) -> Result<Vec<f32>, String> {
        let mut host = vec![0.0f32; n];
        let bytesize = n * std::mem::size_of::<f32>();
        self.api
            .memcpy_d_to_h(host.as_mut_ptr() as *mut c_void, ptr, bytesize)?;
        Ok(host)
    }

    /// Extract the `CudaBuffer` from a `DeviceTensor`.
    fn as_buf(dt: &DeviceTensor) -> &CudaBuffer {
        dt.inner
            .downcast_ref::<CudaBuffer>()
            .expect("CudaBackend: expected CudaBuffer in DeviceTensor")
    }

    /// Wrap a `CudaBuffer` into a `DeviceTensor`.
    fn wrap(buf: CudaBuffer, rows: usize, cols: usize) -> DeviceTensor {
        DeviceTensor {
            rows,
            cols,
            inner: Box::new(buf),
        }
    }

    /// Synchronize the compute stream.
    fn sync(&self) {
        if let Err(e) = self.api.stream_synchronize(self.stream) {
            tracing::warn!(target: "strata::embed", error = %e, "CUDA: stream synchronize failed");
        }
    }

    /// Launch a kernel with the given grid/block configuration and parameters.
    ///
    /// # Safety
    ///
    /// `params` must be a correctly constructed parameter array matching the
    /// kernel signature.
    unsafe fn launch(
        &self,
        func: CUfunction,
        grid: (u32, u32, u32),
        block: (u32, u32, u32),
        shared_mem: u32,
        params: &mut [*mut c_void],
    ) {
        if let Err(e) = self.api.launch_kernel(
            func,
            grid,
            block,
            shared_mem,
            self.stream,
            params.as_mut_ptr(),
        ) {
            tracing::warn!(target: "strata::embed", error = %e, "CUDA: kernel launch failed");
        }
    }

    /// Integer ceiling division.
    fn div_ceil(a: u32, b: u32) -> u32 {
        (a + b - 1) / b
    }
}

// ---------------------------------------------------------------------------
// ComputeBackend implementation
// ---------------------------------------------------------------------------

impl ComputeBackend for CudaBackend {
    fn upload(&self, t: &Tensor) -> DeviceTensor {
        match self.upload_f32(&t.data) {
            Ok(buf) => Self::wrap(buf, t.rows, t.cols),
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: upload failed, falling back to zeros");
                Self::wrap(
                    self.alloc_zeros_f32(t.rows * t.cols)
                        .expect("CUDA: alloc_zeros_f32 failed after upload failure"),
                    t.rows,
                    t.cols,
                )
            }
        }
    }

    fn upload_1d(&self, v: &[f32]) -> DeviceTensor {
        match self.upload_f32(v) {
            Ok(buf) => Self::wrap(buf, 1, v.len()),
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: upload_1d failed");
                Self::wrap(
                    self.alloc_zeros_f32(v.len())
                        .expect("CUDA: alloc_zeros_f32 failed after upload_1d failure"),
                    1,
                    v.len(),
                )
            }
        }
    }

    fn download(&self, dt: &DeviceTensor) -> Tensor {
        self.sync();
        let buf = Self::as_buf(dt);
        let n = dt.rows * dt.cols;
        match self.download_f32(buf.ptr, n) {
            Ok(data) => Tensor {
                data,
                rows: dt.rows,
                cols: dt.cols,
            },
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: download failed, returning zeros");
                Tensor::zeros(dt.rows, dt.cols)
            }
        }
    }

    fn matmul(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        let m = a.rows as u32;
        let k = a.cols as u32;
        let n = b.cols as u32;

        let out = match self.alloc_zeros_f32(a.rows * b.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: matmul alloc failed, returning zeros");
                return self.zeros(a.rows, b.cols);
            }
        };

        let a_buf = Self::as_buf(a);
        let b_buf = Self::as_buf(b);

        // Try cuBLAS first. Row-major C(M,N) = A(M,K) * B(K,N):
        // cuBLAS is column-major, so we compute C^T = B^T * A^T, i.e.:
        // cublasSgemm(OP_N, OP_N, N, M, K, 1, B, N, A, K, 0, C, N)
        if let Some(ref cublas) = self.cublas {
            if cublas
                .sgemm(
                    ffi::CUBLAS_OP_N,
                    ffi::CUBLAS_OP_N,
                    n as i32,
                    m as i32,
                    k as i32,
                    1.0,
                    b_buf.ptr,
                    n as i32,
                    a_buf.ptr,
                    k as i32,
                    0.0,
                    out.ptr,
                    n as i32,
                )
                .is_ok()
            {
                return Self::wrap(out, a.rows, b.cols);
            }
        }

        // Fallback: PTX GEMM kernel
        let mut p_a = a_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_c = out.ptr;
        let mut p_m = m;
        let mut p_k = k;
        let mut p_n = n;

        let mut params: [*mut c_void; 6] = [
            &mut p_a as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
            &mut p_m as *mut _ as *mut c_void,
            &mut p_k as *mut _ as *mut c_void,
            &mut p_n as *mut _ as *mut c_void,
        ];

        let grid = (Self::div_ceil(n, 16), Self::div_ceil(m, 16), 1);
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_gemm, grid, block, 0, &mut params);
        }

        Self::wrap(out, a.rows, b.cols)
    }

    fn matmul_transpose(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        let m = a.rows as u32;
        let k = a.cols as u32;
        let n = b.rows as u32; // B is (N, K) and we treat it as transposed

        let out = match self.alloc_zeros_f32(a.rows * b.rows) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: matmul_transpose alloc failed, returning zeros");
                return self.zeros(a.rows, b.rows);
            }
        };

        let a_buf = Self::as_buf(a);
        let b_buf = Self::as_buf(b);

        // Try cuBLAS first. Row-major C(M,N) = A(M,K) * B(N,K)^T:
        // cuBLAS column-major: C^T = B * A^T, i.e.:
        // cublasSgemm(OP_T, OP_N, N, M, K, 1, B, K, A, K, 0, C, N)
        if let Some(ref cublas) = self.cublas {
            if cublas
                .sgemm(
                    ffi::CUBLAS_OP_T,
                    ffi::CUBLAS_OP_N,
                    n as i32,
                    m as i32,
                    k as i32,
                    1.0,
                    b_buf.ptr,
                    k as i32,
                    a_buf.ptr,
                    k as i32,
                    0.0,
                    out.ptr,
                    n as i32,
                )
                .is_ok()
            {
                return Self::wrap(out, a.rows, b.rows);
            }
        }

        // Fallback: PTX GEMM transpose kernel
        let mut p_a = a_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_c = out.ptr;
        let mut p_m = m;
        let mut p_k = k;
        let mut p_n = n;

        let mut params: [*mut c_void; 6] = [
            &mut p_a as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
            &mut p_m as *mut _ as *mut c_void,
            &mut p_k as *mut _ as *mut c_void,
            &mut p_n as *mut _ as *mut c_void,
        ];

        let grid = (Self::div_ceil(n, 16), Self::div_ceil(m, 16), 1);
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_gemm_transpose, grid, block, 0, &mut params);
        }

        Self::wrap(out, a.rows, b.rows)
    }

    fn add_bias(&self, t: &mut DeviceTensor, bias: &DeviceTensor) {
        let rows = t.rows as u32;
        let cols = t.cols as u32;

        let t_buf = Self::as_buf(t);
        let bias_buf = Self::as_buf(bias);

        let mut p_t = t_buf.ptr;
        let mut p_bias = bias_buf.ptr;
        let mut p_rows = rows;
        let mut p_cols = cols;

        let mut params: [*mut c_void; 4] = [
            &mut p_t as *mut _ as *mut c_void,
            &mut p_bias as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_cols as *mut _ as *mut c_void,
        ];

        let grid = (rows, Self::div_ceil(cols, 256), 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_add_bias, grid, block, 0, &mut params);
        }
    }

    fn add_tensor(&self, a: &DeviceTensor, b: &DeviceTensor) -> DeviceTensor {
        let n = (a.rows * a.cols) as u32;

        let out = match self.alloc_zeros_f32(a.rows * a.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: add_tensor alloc failed, returning zeros");
                return self.zeros(a.rows, a.cols);
            }
        };

        let a_buf = Self::as_buf(a);
        let b_buf = Self::as_buf(b);

        let mut p_a = a_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_c = out.ptr;
        let mut p_n = n;

        let mut params: [*mut c_void; 4] = [
            &mut p_a as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
            &mut p_n as *mut _ as *mut c_void,
        ];

        let grid = (Self::div_ceil(n, 256), 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_add_tensor, grid, block, 0, &mut params);
        }

        Self::wrap(out, a.rows, a.cols)
    }

    fn gelu(&self, t: &DeviceTensor) -> DeviceTensor {
        let n = (t.rows * t.cols) as u32;

        let out = match self.alloc_zeros_f32(t.rows * t.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: gelu alloc failed, returning zeros");
                return self.zeros(t.rows, t.cols);
            }
        };

        let t_buf = Self::as_buf(t);

        let mut p_in = t_buf.ptr;
        let mut p_out = out.ptr;
        let mut p_n = n;

        let mut params: [*mut c_void; 3] = [
            &mut p_in as *mut _ as *mut c_void,
            &mut p_out as *mut _ as *mut c_void,
            &mut p_n as *mut _ as *mut c_void,
        ];

        let grid = (Self::div_ceil(n, 256), 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_gelu, grid, block, 0, &mut params);
        }

        Self::wrap(out, t.rows, t.cols)
    }

    fn layer_norm(
        &self,
        t: &DeviceTensor,
        w: &DeviceTensor,
        b: &DeviceTensor,
        eps: f32,
    ) -> DeviceTensor {
        let rows = t.rows as u32;
        let cols = t.cols as u32;

        let out = match self.alloc_zeros_f32(t.rows * t.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: layer_norm alloc failed, returning zeros");
                return self.zeros(t.rows, t.cols);
            }
        };

        let t_buf = Self::as_buf(t);
        let w_buf = Self::as_buf(w);
        let b_buf = Self::as_buf(b);

        let mut p_in = t_buf.ptr;
        let mut p_out = out.ptr;
        let mut p_w = w_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_rows = rows;
        let mut p_cols = cols;
        let mut p_eps = eps;

        let mut params: [*mut c_void; 7] = [
            &mut p_in as *mut _ as *mut c_void,
            &mut p_out as *mut _ as *mut c_void,
            &mut p_w as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_cols as *mut _ as *mut c_void,
            &mut p_eps as *mut _ as *mut c_void,
        ];

        let grid = (rows, 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_layer_norm, grid, block, 0, &mut params);
        }

        Self::wrap(out, t.rows, t.cols)
    }

    fn softmax_rows(&self, t: &mut DeviceTensor) {
        let rows = t.rows as u32;
        let cols = t.cols as u32;

        let t_buf = Self::as_buf(t);

        let mut p_data = t_buf.ptr;
        let mut p_rows = rows;
        let mut p_cols = cols;

        let mut params: [*mut c_void; 3] = [
            &mut p_data as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_cols as *mut _ as *mut c_void,
        ];

        let grid = (rows, 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_softmax_rows, grid, block, 0, &mut params);
        }
    }

    fn scale(&self, t: &mut DeviceTensor, factor: f32) {
        let n = (t.rows * t.cols) as u32;

        let t_buf = Self::as_buf(t);

        let mut p_t = t_buf.ptr;
        let mut p_factor = factor;
        let mut p_n = n;

        let mut params: [*mut c_void; 3] = [
            &mut p_t as *mut _ as *mut c_void,
            &mut p_factor as *mut _ as *mut c_void,
            &mut p_n as *mut _ as *mut c_void,
        ];

        let grid = (Self::div_ceil(n, 256), 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_scale, grid, block, 0, &mut params);
        }
    }

    fn slice_columns(&self, t: &DeviceTensor, start: usize, end: usize) -> DeviceTensor {
        let rows = t.rows as u32;
        let src_cols = t.cols as u32;
        let dst_cols = (end - start) as u32;
        let col_start = start as u32;

        let out = match self.alloc_zeros_f32(t.rows * (end - start)) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: slice_columns alloc failed, returning zeros");
                return self.zeros(t.rows, end - start);
            }
        };

        let t_buf = Self::as_buf(t);

        let mut p_src = t_buf.ptr;
        let mut p_dst = out.ptr;
        let mut p_rows = rows;
        let mut p_src_cols = src_cols;
        let mut p_dst_cols = dst_cols;
        let mut p_col_start = col_start;

        let mut params: [*mut c_void; 6] = [
            &mut p_src as *mut _ as *mut c_void,
            &mut p_dst as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_src_cols as *mut _ as *mut c_void,
            &mut p_dst_cols as *mut _ as *mut c_void,
            &mut p_col_start as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(dst_cols, 16),
            Self::div_ceil(rows, 16),
            1,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_slice_columns, grid, block, 0, &mut params);
        }

        Self::wrap(out, t.rows, end - start)
    }

    fn scatter_columns(&self, dst: &mut DeviceTensor, src: &DeviceTensor, col_offset: usize) {
        let rows = src.rows as u32;
        let dst_cols = dst.cols as u32;
        let src_cols = src.cols as u32;
        let col_off = col_offset as u32;

        let dst_buf = Self::as_buf(dst);
        let src_buf = Self::as_buf(src);

        let mut p_dst = dst_buf.ptr;
        let mut p_src = src_buf.ptr;
        let mut p_rows = rows;
        let mut p_dst_cols = dst_cols;
        let mut p_src_cols = src_cols;
        let mut p_col_offset = col_off;

        let mut params: [*mut c_void; 6] = [
            &mut p_dst as *mut _ as *mut c_void,
            &mut p_src as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_dst_cols as *mut _ as *mut c_void,
            &mut p_src_cols as *mut _ as *mut c_void,
            &mut p_col_offset as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(src_cols, 16),
            Self::div_ceil(rows, 16),
            1,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_scatter_columns, grid, block, 0, &mut params);
        }
    }

    fn zeros(&self, rows: usize, cols: usize) -> DeviceTensor {
        match self.alloc_zeros_f32(rows * cols) {
            Ok(buf) => Self::wrap(buf, rows, cols),
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: zeros alloc failed");
                // This is a critical path; panic is acceptable here.
                panic!("CUDA: failed to allocate zero tensor: {}", e);
            }
        }
    }

    fn upload_mask(&self, mask: &[u32]) -> DeviceTensor {
        match self.upload_u32(mask) {
            Ok(buf) => Self::wrap(buf, 1, mask.len()),
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: upload_mask failed");
                panic!("CUDA: failed to upload mask: {}", e);
            }
        }
    }

    fn apply_attention_mask(&self, scores: &mut DeviceTensor, mask: &DeviceTensor) {
        let rows = scores.rows as u32;
        let cols = scores.cols as u32;

        let scores_buf = Self::as_buf(scores);
        let mask_buf = Self::as_buf(mask);

        let mut p_scores = scores_buf.ptr;
        let mut p_mask = mask_buf.ptr;
        let mut p_rows = rows;
        let mut p_cols = cols;

        let mut params: [*mut c_void; 4] = [
            &mut p_scores as *mut _ as *mut c_void,
            &mut p_mask as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_cols as *mut _ as *mut c_void,
        ];

        let grid = (rows, Self::div_ceil(cols, 256), 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_attention_mask, grid, block, 0, &mut params);
        }
    }

    fn mean_pool(&self, hidden: &DeviceTensor, mask: &DeviceTensor) -> Vec<f32> {
        let rows = hidden.rows as u32;
        let cols = hidden.cols as u32;

        // Allocate output on device (1 row)
        let out_buf = match self.alloc_zeros_f32(hidden.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: mean_pool output alloc failed");
                return vec![0.0f32; hidden.cols];
            }
        };

        let hidden_buf = Self::as_buf(hidden);
        let mask_buf = Self::as_buf(mask);

        let mut p_hidden = hidden_buf.ptr;
        let mut p_mask = mask_buf.ptr;
        let mut p_output = out_buf.ptr;
        let mut p_rows = rows;
        let mut p_cols = cols;

        let mut params: [*mut c_void; 5] = [
            &mut p_hidden as *mut _ as *mut c_void,
            &mut p_mask as *mut _ as *mut c_void,
            &mut p_output as *mut _ as *mut c_void,
            &mut p_rows as *mut _ as *mut c_void,
            &mut p_cols as *mut _ as *mut c_void,
        ];

        let grid = (1, 1, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_mean_pool, grid, block, 0, &mut params);
        }

        // Synchronize and download
        self.sync();
        match self.download_f32(out_buf.ptr, hidden.cols) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: mean_pool download failed");
                vec![0.0f32; hidden.cols]
            }
        }
    }

    fn batched_matmul_transpose(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor {
        let k = a.cols as u32;
        let s = seq_len as u32;

        let out = match self.alloc_zeros_f32(batch_size * seq_len * seq_len) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: batched_matmul_transpose alloc failed, returning zeros");
                return self.zeros(batch_size * seq_len, seq_len);
            }
        };

        let a_buf = Self::as_buf(a);
        let b_buf = Self::as_buf(b);

        // Try cuBLAS strided batched first.
        // Row-major C[b](S,S) = A[b](S,K) * B[b](S,K)^T:
        // cuBLAS column-major: C[b]^T = B[b] * A[b]^T
        // cublasSgemmStridedBatched(OP_T, OP_N, S, S, K, 1, B, K, S*K, A, K, S*K, 0, C, S, S*S, batch)
        if let Some(ref cublas) = self.cublas {
            let stride_ab = (seq_len * a.cols) as i64;
            let stride_c = (seq_len * seq_len) as i64;
            if cublas
                .sgemm_strided_batched(
                    ffi::CUBLAS_OP_T,
                    ffi::CUBLAS_OP_N,
                    s as i32,
                    s as i32,
                    k as i32,
                    1.0,
                    b_buf.ptr,
                    k as i32,
                    stride_ab,
                    a_buf.ptr,
                    k as i32,
                    stride_ab,
                    0.0,
                    out.ptr,
                    s as i32,
                    stride_c,
                    batch_size as i32,
                )
                .is_ok()
            {
                return Self::wrap(out, batch_size * seq_len, seq_len);
            }
        }

        // Fallback: PTX batched GEMM transpose kernel
        let mut p_a = a_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_c = out.ptr;
        let mut p_s = s;
        let mut p_k = k;

        let mut params: [*mut c_void; 5] = [
            &mut p_a as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
            &mut p_s as *mut _ as *mut c_void,
            &mut p_k as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(s, 16),
            Self::div_ceil(s, 16),
            batch_size as u32,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_batched_gemm_transpose, grid, block, 0, &mut params);
        }

        Self::wrap(out, batch_size * seq_len, seq_len)
    }

    fn batched_matmul(
        &self,
        a: &DeviceTensor,
        b: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> DeviceTensor {
        let k = b.cols as u32;
        let s = seq_len as u32;

        let out = match self.alloc_zeros_f32(batch_size * seq_len * b.cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: batched_matmul alloc failed, returning zeros");
                return self.zeros(batch_size * seq_len, b.cols);
            }
        };

        let a_buf = Self::as_buf(a);
        let b_buf = Self::as_buf(b);

        // Try cuBLAS strided batched first.
        // Row-major C[b](S,K) = A[b](S,S) * B[b](S,K):
        // cuBLAS column-major: C[b]^T = B[b]^T * A[b]^T
        // cublasSgemmStridedBatched(OP_N, OP_N, K, S, S, 1, B, K, S*K, A, S, S*S, 0, C, K, S*K, batch)
        if let Some(ref cublas) = self.cublas {
            let stride_a = (seq_len * seq_len) as i64;
            let stride_b = (seq_len * b.cols) as i64;
            let stride_c = stride_b;
            if cublas
                .sgemm_strided_batched(
                    ffi::CUBLAS_OP_N,
                    ffi::CUBLAS_OP_N,
                    k as i32,
                    s as i32,
                    s as i32,
                    1.0,
                    b_buf.ptr,
                    k as i32,
                    stride_b,
                    a_buf.ptr,
                    s as i32,
                    stride_a,
                    0.0,
                    out.ptr,
                    k as i32,
                    stride_c,
                    batch_size as i32,
                )
                .is_ok()
            {
                return Self::wrap(out, batch_size * seq_len, b.cols);
            }
        }

        // Fallback: PTX batched GEMM kernel
        let mut p_a = a_buf.ptr;
        let mut p_b = b_buf.ptr;
        let mut p_c = out.ptr;
        let mut p_s = s;
        let mut p_k = k;

        let mut params: [*mut c_void; 5] = [
            &mut p_a as *mut _ as *mut c_void,
            &mut p_b as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
            &mut p_s as *mut _ as *mut c_void,
            &mut p_k as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(k, 16),
            Self::div_ceil(s, 16),
            batch_size as u32,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_batched_gemm, grid, block, 0, &mut params);
        }

        Self::wrap(out, batch_size * seq_len, b.cols)
    }

    fn batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) {
        let total_rows = (batch_size * seq_len) as u32;
        let s = seq_len as u32;

        let scores_buf = Self::as_buf(scores);
        let mask_buf = Self::as_buf(mask);

        let mut p_scores = scores_buf.ptr;
        let mut p_mask = mask_buf.ptr;
        let mut p_total_rows = total_rows;
        let mut p_s = s;

        let mut params: [*mut c_void; 4] = [
            &mut p_scores as *mut _ as *mut c_void,
            &mut p_mask as *mut _ as *mut c_void,
            &mut p_total_rows as *mut _ as *mut c_void,
            &mut p_s as *mut _ as *mut c_void,
        ];

        let grid = (total_rows, Self::div_ceil(s, 256), 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_batched_attention_mask, grid, block, 0, &mut params);
        }
    }

    fn batched_mean_pool(
        &self,
        hidden: &DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
    ) -> Vec<Vec<f32>> {
        let cols = hidden.cols;
        let s = seq_len as u32;
        let c = cols as u32;

        // Allocate output on device (batch_size * cols)
        let out_buf = match self.alloc_zeros_f32(batch_size * cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: batched_mean_pool output alloc failed");
                return vec![vec![0.0f32; cols]; batch_size];
            }
        };

        let hidden_buf = Self::as_buf(hidden);
        let mask_buf = Self::as_buf(mask);

        let mut p_hidden = hidden_buf.ptr;
        let mut p_mask = mask_buf.ptr;
        let mut p_output = out_buf.ptr;
        let mut p_s = s;
        let mut p_c = c;

        let mut params: [*mut c_void; 5] = [
            &mut p_hidden as *mut _ as *mut c_void,
            &mut p_mask as *mut _ as *mut c_void,
            &mut p_output as *mut _ as *mut c_void,
            &mut p_s as *mut _ as *mut c_void,
            &mut p_c as *mut _ as *mut c_void,
        ];

        let grid = (1, batch_size as u32, 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(self.fn_batched_mean_pool, grid, block, 0, &mut params);
        }

        // Synchronize and download
        self.sync();
        match self.download_f32(out_buf.ptr, batch_size * cols) {
            Ok(data) => data.chunks(cols).map(|chunk| chunk.to_vec()).collect(),
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: batched_mean_pool download failed");
                vec![vec![0.0f32; cols]; batch_size]
            }
        }
    }

    fn transpose_heads(
        &self,
        t: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
        head_dim: usize,
    ) -> DeviceTensor {
        let total_out_rows = batch_size * num_heads * seq_len;
        let out = match self.alloc_zeros_f32(total_out_rows * head_dim) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: transpose_heads alloc failed");
                return self.zeros(total_out_rows, head_dim);
            }
        };

        let src_buf = Self::as_buf(t);
        let total_in_rows = (batch_size * seq_len) as u32;

        let mut p_src = src_buf.ptr;
        let mut p_dst = out.ptr;
        let mut p_bs = batch_size as u32;
        let mut p_sl = seq_len as u32;
        let mut p_nh = num_heads as u32;
        let mut p_hd = head_dim as u32;

        let mut params: [*mut c_void; 6] = [
            &mut p_src as *mut _ as *mut c_void,
            &mut p_dst as *mut _ as *mut c_void,
            &mut p_bs as *mut _ as *mut c_void,
            &mut p_sl as *mut _ as *mut c_void,
            &mut p_nh as *mut _ as *mut c_void,
            &mut p_hd as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(head_dim as u32, 16),
            Self::div_ceil(total_in_rows, 16),
            num_heads as u32,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_transpose_heads, grid, block, 0, &mut params);
        }

        Self::wrap(out, total_out_rows, head_dim)
    }

    fn untranspose_heads(
        &self,
        t: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
        head_dim: usize,
    ) -> DeviceTensor {
        let total_out_rows = batch_size * seq_len;
        let out_cols = num_heads * head_dim;
        let out = match self.alloc_zeros_f32(total_out_rows * out_cols) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!(target: "strata::embed", error = %e, "CUDA: untranspose_heads alloc failed");
                return self.zeros(total_out_rows, out_cols);
            }
        };

        let src_buf = Self::as_buf(t);

        let mut p_src = src_buf.ptr;
        let mut p_dst = out.ptr;
        let mut p_bs = batch_size as u32;
        let mut p_sl = seq_len as u32;
        let mut p_nh = num_heads as u32;
        let mut p_hd = head_dim as u32;

        let mut params: [*mut c_void; 6] = [
            &mut p_src as *mut _ as *mut c_void,
            &mut p_dst as *mut _ as *mut c_void,
            &mut p_bs as *mut _ as *mut c_void,
            &mut p_sl as *mut _ as *mut c_void,
            &mut p_nh as *mut _ as *mut c_void,
            &mut p_hd as *mut _ as *mut c_void,
        ];

        let grid = (
            Self::div_ceil(head_dim as u32, 16),
            Self::div_ceil(total_out_rows as u32, 16),
            num_heads as u32,
        );
        let block = (16, 16, 1);
        unsafe {
            self.launch(self.fn_untranspose_heads, grid, block, 0, &mut params);
        }

        Self::wrap(out, total_out_rows, out_cols)
    }

    fn multi_head_batched_attention_mask(
        &self,
        scores: &mut DeviceTensor,
        mask: &DeviceTensor,
        batch_size: usize,
        seq_len: usize,
        num_heads: usize,
    ) {
        let total_rows = (batch_size * num_heads * seq_len) as u32;
        let s = seq_len as u32;
        let h = num_heads as u32;

        let scores_buf = Self::as_buf(scores);
        let mask_buf = Self::as_buf(mask);

        let mut p_scores = scores_buf.ptr;
        let mut p_mask = mask_buf.ptr;
        let mut p_total_rows = total_rows;
        let mut p_s = s;
        let mut p_h = h;

        let mut params: [*mut c_void; 5] = [
            &mut p_scores as *mut _ as *mut c_void,
            &mut p_mask as *mut _ as *mut c_void,
            &mut p_total_rows as *mut _ as *mut c_void,
            &mut p_s as *mut _ as *mut c_void,
            &mut p_h as *mut _ as *mut c_void,
        ];

        let grid = (total_rows, Self::div_ceil(s, 256), 1);
        let block = (256, 1, 1);
        unsafe {
            self.launch(
                self.fn_multi_head_batched_attention_mask,
                grid,
                block,
                0,
                &mut params,
            );
        }
    }

    fn name(&self) -> &'static str {
        "CUDA"
    }
}

impl Drop for CudaBackend {
    fn drop(&mut self) {
        // Synchronize before cleanup to ensure all work is complete.
        let _ = self.api.stream_synchronize(self.stream);

        // Drop cuBLAS BEFORE destroying the stream it's bound to.
        // cublasDestroy may synchronize on the stream internally.
        drop(self.cublas.take());

        if let Err(e) = self.api.module_unload(self.module) {
            tracing::warn!(target: "strata::embed", error = %e, "CUDA: failed to unload module");
        }
        if let Err(e) = self.api.stream_destroy(self.stream) {
            tracing::warn!(target: "strata::embed", error = %e, "CUDA: failed to destroy stream");
        }
        // Context destruction is handled by CudaApi::drop (which destroys self.api.ctx).
        // We do NOT destroy the context here because the Arc<CudaApi> may still be
        // held by CudaBuffer instances that need to call cuMemFree.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::cpu_backend::CpuBackend;

    fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, f32::max)
    }

    fn try_cuda() -> Option<CudaBackend> {
        CudaBackend::try_new().ok()
    }

    #[test]
    fn cuda_vs_cpu_matmul() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        // A(4, 8) * B(8, 6) = C(4, 6)
        let a_data: Vec<f32> = (0..32).map(|i| (i as f32) * 0.1).collect();
        let b_data: Vec<f32> = (0..48).map(|i| (i as f32) * 0.05 - 1.0).collect();

        let a_cpu = Tensor::from_slice(&a_data, 4, 8);
        let b_cpu = Tensor::from_slice(&b_data, 8, 6);

        let c_cpu_t = cpu.matmul(&cpu.upload(&a_cpu), &cpu.upload(&b_cpu));
        let c_cpu_result = cpu.download(&c_cpu_t);

        let c_cuda_t = cuda.matmul(&cuda.upload(&a_cpu), &cuda.upload(&b_cpu));
        let c_cuda_result = cuda.download(&c_cuda_t);

        let diff = max_abs_diff(&c_cpu_result.data, &c_cuda_result.data);
        eprintln!("matmul max diff: {diff}");
        eprintln!("CPU first 6: {:?}", &c_cpu_result.data[..6]);
        eprintln!("CUDA first 6: {:?}", &c_cuda_result.data[..6]);
        assert!(diff < 1e-3, "matmul: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_matmul_transpose() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        // A(4, 8) * B(6, 8)^T = C(4, 6)
        let a_data: Vec<f32> = (0..32).map(|i| (i as f32) * 0.1).collect();
        let b_data: Vec<f32> = (0..48).map(|i| (i as f32) * 0.05 - 1.0).collect();

        let a_cpu = Tensor::from_slice(&a_data, 4, 8);
        let b_cpu = Tensor::from_slice(&b_data, 6, 8);

        let c_cpu_t = cpu.matmul_transpose(&cpu.upload(&a_cpu), &cpu.upload(&b_cpu));
        let c_cpu_result = cpu.download(&c_cpu_t);

        let c_cuda_t = cuda.matmul_transpose(&cuda.upload(&a_cpu), &cuda.upload(&b_cpu));
        let c_cuda_result = cuda.download(&c_cuda_t);

        let diff = max_abs_diff(&c_cpu_result.data, &c_cuda_result.data);
        eprintln!("matmul_transpose max diff: {diff}");
        eprintln!("CPU first 6: {:?}", &c_cpu_result.data[..6]);
        eprintln!("CUDA first 6: {:?}", &c_cuda_result.data[..6]);
        assert!(diff < 1e-3, "matmul_transpose: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_layer_norm() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        let data: Vec<f32> = (0..24).map(|i| (i as f32) * 0.3 - 2.0).collect();
        let w_data = vec![1.0f32; 6];
        let b_data = vec![0.0f32; 6];

        let t = Tensor::from_slice(&data, 4, 6);
        let w = Tensor::from_slice(&w_data, 1, 6);
        let b_t = Tensor::from_slice(&b_data, 1, 6);

        let cpu_r = cpu.download(&cpu.layer_norm(
            &cpu.upload(&t),
            &cpu.upload(&w),
            &cpu.upload(&b_t),
            1e-5,
        ));

        let cuda_r = cuda.download(&cuda.layer_norm(
            &cuda.upload(&t),
            &cuda.upload(&w),
            &cuda.upload(&b_t),
            1e-5,
        ));

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("layer_norm max diff: {diff}");
        eprintln!("CPU first 6: {:?}", &cpu_r.data[..6]);
        eprintln!("CUDA first 6: {:?}", &cuda_r.data[..6]);
        assert!(diff < 1e-3, "layer_norm: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_gelu() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        let data: Vec<f32> = (0..16).map(|i| (i as f32) * 0.5 - 4.0).collect();
        let t = Tensor::from_slice(&data, 4, 4);

        let cpu_r = cpu.download(&cpu.gelu(&cpu.upload(&t)));
        let cuda_r = cuda.download(&cuda.gelu(&cuda.upload(&t)));

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("gelu max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r.data);
        eprintln!("CUDA: {:?}", &cuda_r.data);
        assert!(diff < 1e-3, "gelu: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_softmax() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        let data: Vec<f32> = (0..16).map(|i| (i as f32) * 0.5 - 4.0).collect();
        let t = Tensor::from_slice(&data, 4, 4);

        let mut cpu_t = cpu.upload(&t);
        cpu.softmax_rows(&mut cpu_t);
        let cpu_r = cpu.download(&cpu_t);

        let mut cuda_t = cuda.upload(&t);
        cuda.softmax_rows(&mut cuda_t);
        let cuda_r = cuda.download(&cuda_t);

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("softmax max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r.data);
        eprintln!("CUDA: {:?}", &cuda_r.data);
        assert!(diff < 1e-3, "softmax: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_add_bias() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        let data: Vec<f32> = (0..12).map(|i| (i as f32) * 0.1).collect();
        let bias: Vec<f32> = vec![1.0, -1.0, 0.5];
        let t = Tensor::from_slice(&data, 4, 3);
        let b_t = Tensor::from_slice(&bias, 1, 3);

        let mut cpu_t = cpu.upload(&t);
        cpu.add_bias(&mut cpu_t, &cpu.upload(&b_t));
        let cpu_r = cpu.download(&cpu_t);

        let mut cuda_t = cuda.upload(&t);
        cuda.add_bias(&mut cuda_t, &cuda.upload(&b_t));
        let cuda_r = cuda.download(&cuda_t);

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("add_bias max diff: {diff}");
        assert!(diff < 1e-5, "add_bias: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_mean_pool() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        // 3 tokens, 4 dims. Mask: [1, 1, 0] (only first 2 active)
        let data: Vec<f32> = (0..12).map(|i| (i as f32) * 0.1).collect();
        let t = Tensor::from_slice(&data, 3, 4);
        let mask = vec![1u32, 1, 0];

        let cpu_t = cpu.upload(&t);
        let cpu_mask = cpu.upload_mask(&mask);
        let cpu_r = cpu.mean_pool(&cpu_t, &cpu_mask);

        let cuda_t = cuda.upload(&t);
        let cuda_mask = cuda.upload_mask(&mask);
        let cuda_r = cuda.mean_pool(&cuda_t, &cuda_mask);

        let diff = max_abs_diff(&cpu_r, &cuda_r);
        eprintln!("mean_pool max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r);
        eprintln!("CUDA: {:?}", &cuda_r);
        assert!(diff < 1e-4, "mean_pool: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_add_tensor() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        let a_data: Vec<f32> = (0..12).map(|i| i as f32 * 0.1).collect();
        let b_data: Vec<f32> = (0..12).map(|i| i as f32 * -0.2 + 1.0).collect();
        let a = Tensor::from_slice(&a_data, 3, 4);
        let b = Tensor::from_slice(&b_data, 3, 4);

        let cpu_r = cpu.download(&cpu.add_tensor(&cpu.upload(&a), &cpu.upload(&b)));
        let cuda_r = cuda.download(&cuda.add_tensor(&cuda.upload(&a), &cuda.upload(&b)));

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("add_tensor max diff: {diff}");
        assert!(diff < 1e-5, "add_tensor: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_scale() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        let data: Vec<f32> = (0..12).map(|i| i as f32 * 0.3).collect();
        let t = Tensor::from_slice(&data, 3, 4);

        let mut cpu_t = cpu.upload(&t);
        cpu.scale(&mut cpu_t, 0.1767766953);
        let cpu_r = cpu.download(&cpu_t);

        let mut cuda_t = cuda.upload(&t);
        cuda.scale(&mut cuda_t, 0.1767766953);
        let cuda_r = cuda.download(&cuda_t);

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("scale max diff: {diff}");
        assert!(diff < 1e-5, "scale: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_batched_matmul_transpose() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        // batch_size=2, seq_len=3, K=4
        // A: (2*3, 4) = (6, 4), B: (6, 4)
        // Result: (6, 3) — for each batch, (3,4)*(3,4)^T = (3,3)
        let a_data: Vec<f32> = (0..24).map(|i| (i as f32) * 0.1).collect();
        let b_data: Vec<f32> = (0..24).map(|i| (i as f32) * 0.05 - 0.5).collect();

        let a = Tensor::from_slice(&a_data, 6, 4);
        let b = Tensor::from_slice(&b_data, 6, 4);

        let cpu_r = cpu.download(&cpu.batched_matmul_transpose(
            &cpu.upload(&a), &cpu.upload(&b), 2, 3));
        let cuda_r = cuda.download(&cuda.batched_matmul_transpose(
            &cuda.upload(&a), &cuda.upload(&b), 2, 3));

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("batched_matmul_transpose max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r.data);
        eprintln!("CUDA: {:?}", &cuda_r.data);
        assert!(diff < 1e-3, "batched_matmul_transpose: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_batched_matmul() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        // batch_size=2, seq_len=3, K=4
        // A: (6, 3) — scores, B: (6, 4) — values
        // Result: (6, 4) — for each batch, (3,3)*(3,4) = (3,4)
        let a_data: Vec<f32> = (0..18).map(|i| (i as f32) * 0.1).collect();
        let b_data: Vec<f32> = (0..24).map(|i| (i as f32) * 0.05).collect();

        let a = Tensor::from_slice(&a_data, 6, 3);
        let b = Tensor::from_slice(&b_data, 6, 4);

        let cpu_r = cpu.download(&cpu.batched_matmul(
            &cpu.upload(&a), &cpu.upload(&b), 2, 3));
        let cuda_r = cuda.download(&cuda.batched_matmul(
            &cuda.upload(&a), &cuda.upload(&b), 2, 3));

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("batched_matmul max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r.data);
        eprintln!("CUDA: {:?}", &cuda_r.data);
        assert!(diff < 1e-3, "batched_matmul: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_batched_attention_mask() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        // batch_size=2, seq_len=3
        // scores: (6, 3), mask: [1,1,0, 1,1,1]
        let data: Vec<f32> = (0..18).map(|i| (i as f32) * 0.5).collect();
        let t = Tensor::from_slice(&data, 6, 3);
        let mask = vec![1u32, 1, 0, 1, 1, 1];

        let mut cpu_t = cpu.upload(&t);
        let cpu_mask = cpu.upload_mask(&mask);
        cpu.batched_attention_mask(&mut cpu_t, &cpu_mask, 2, 3);
        let cpu_r = cpu.download(&cpu_t);

        let mut cuda_t = cuda.upload(&t);
        let cuda_mask = cuda.upload_mask(&mask);
        cuda.batched_attention_mask(&mut cuda_t, &cuda_mask, 2, 3);
        let cuda_r = cuda.download(&cuda_t);

        let diff = max_abs_diff(&cpu_r.data, &cuda_r.data);
        eprintln!("batched_attention_mask max diff: {diff}");
        eprintln!("CPU: {:?}", &cpu_r.data);
        eprintln!("CUDA: {:?}", &cuda_r.data);
        assert!(diff < 1e-5, "batched_attention_mask: max abs diff = {diff}");
    }

    #[test]
    fn cuda_vs_cpu_batched_mean_pool() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => { return; }
        };
        let cpu = CpuBackend;

        // batch_size=2, seq_len=3, dim=4
        // hidden: (6, 4), mask: [1,1,0, 1,1,1]
        let data: Vec<f32> = (0..24).map(|i| (i as f32) * 0.1).collect();
        let t = Tensor::from_slice(&data, 6, 4);
        let mask = vec![1u32, 1, 0, 1, 1, 1];

        let cpu_r = cpu.batched_mean_pool(
            &cpu.upload(&t), &cpu.upload_mask(&mask), 2, 3);
        let cuda_r = cuda.batched_mean_pool(
            &cuda.upload(&t), &cuda.upload_mask(&mask), 2, 3);

        eprintln!("batched_mean_pool:");
        for (i, (c, g)) in cpu_r.iter().zip(cuda_r.iter()).enumerate() {
            let diff = max_abs_diff(c, g);
            eprintln!("  batch {i}: CPU={c:?} CUDA={g:?} diff={diff}");
            assert!(diff < 1e-4, "batched_mean_pool batch {i}: diff = {diff}");
        }
    }

    /// Full-pipeline comparison: embed a text with CPU vs CUDA backend using actual model weights.
    #[test]
    fn cuda_vs_cpu_full_embed() {
        let cuda = match CudaBackend::try_new() {
            Ok(b) => b,
            Err(_) => { return; }
        };

        let home = std::env::var("HOME").unwrap();
        let model_dir = std::path::PathBuf::from(home)
            .join(".stratadb/models/minilm-l6-v2");
        let safetensors_path = model_dir.join("model.safetensors");
        let vocab_path = model_dir.join("vocab.txt");

        if !safetensors_path.exists() {
            eprintln!("Model not found at {:?}, skipping", model_dir);
            return;
        }

        let safetensors_bytes = std::fs::read(&safetensors_path).unwrap();
        let vocab_text = std::fs::read_to_string(&vocab_path).unwrap();

        use crate::embed::model::EmbedModel;

        // Load with CPU backend
        let cpu_backend: Arc<dyn ComputeBackend> = Arc::new(CpuBackend);
        let cpu_model =
            EmbedModel::load_with_backend_for_test(&safetensors_bytes, &vocab_text, cpu_backend)
                .unwrap();

        // Load with CUDA backend
        let cuda_backend: Arc<dyn ComputeBackend> = Arc::new(cuda);
        let cuda_model =
            EmbedModel::load_with_backend_for_test(&safetensors_bytes, &vocab_text, cuda_backend)
                .unwrap();

        let text = "This is a test sentence for embedding comparison between CPU and CUDA backends.";

        let cpu_result = cpu_model.embed(text);
        let cuda_result = cuda_model.embed(text);

        assert_eq!(cpu_result.len(), cuda_result.len());
        let diff = max_abs_diff(&cpu_result, &cuda_result);
        eprintln!("Full embed max diff: {diff}");
        eprintln!("CPU first 8: {:?}", &cpu_result[..8]);
        eprintln!("CUDA first 8: {:?}", &cuda_result[..8]);

        // Compute cosine similarity
        let dot: f32 = cpu_result
            .iter()
            .zip(cuda_result.iter())
            .map(|(a, b)| a * b)
            .sum();
        let norm_a: f32 = cpu_result.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = cuda_result.iter().map(|x| x * x).sum::<f32>().sqrt();
        let cosine_sim = dot / (norm_a * norm_b);
        eprintln!("Cosine similarity: {cosine_sim}");

        assert!(
            cosine_sim > 0.99,
            "Full embed: cosine sim = {cosine_sim} (expected > 0.99)"
        );
    }

    /// Full-pipeline comparison for batched embedding.
    #[test]
    fn cuda_vs_cpu_full_embed_batch() {
        let cuda = match CudaBackend::try_new() {
            Ok(b) => b,
            Err(_) => { return; }
        };

        let home = std::env::var("HOME").unwrap();
        let model_dir = std::path::PathBuf::from(home)
            .join(".stratadb/models/minilm-l6-v2");
        let safetensors_path = model_dir.join("model.safetensors");
        let vocab_path = model_dir.join("vocab.txt");

        if !safetensors_path.exists() {
            eprintln!("Model not found, skipping");
            return;
        }

        let safetensors_bytes = std::fs::read(&safetensors_path).unwrap();
        let vocab_text = std::fs::read_to_string(&vocab_path).unwrap();

        use crate::embed::model::EmbedModel;

        let cpu_backend: Arc<dyn ComputeBackend> = Arc::new(CpuBackend);
        let cpu_model =
            EmbedModel::load_with_backend_for_test(&safetensors_bytes, &vocab_text, cpu_backend)
                .unwrap();

        let cuda_backend: Arc<dyn ComputeBackend> = Arc::new(cuda);
        let cuda_model =
            EmbedModel::load_with_backend_for_test(&safetensors_bytes, &vocab_text, cuda_backend)
                .unwrap();

        let texts = vec![
            "The quick brown fox jumps over the lazy dog.",
            "Machine learning is a subset of artificial intelligence.",
            "Rust programming language focuses on safety and performance.",
            "Cats are wonderful pets.",
        ];
        let text_refs: Vec<&str> = texts.iter().map(|s| &**s).collect();

        let cpu_results = cpu_model.embed_batch(&text_refs);
        let cuda_results = cuda_model.embed_batch(&text_refs);

        assert_eq!(cpu_results.len(), cuda_results.len());
        for (i, (cpu_vec, cuda_vec)) in cpu_results.iter().zip(cuda_results.iter()).enumerate() {
            let diff = max_abs_diff(cpu_vec, cuda_vec);
            let dot: f32 = cpu_vec.iter().zip(cuda_vec.iter()).map(|(a, b)| a * b).sum();
            let norm_a: f32 = cpu_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = cuda_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            let cosine_sim = dot / (norm_a * norm_b);
            eprintln!("Batch item {i}: max_diff={diff}, cosine_sim={cosine_sim}");
            eprintln!("  CPU first 4: {:?}", &cpu_vec[..4]);
            eprintln!("  CUDA first 4: {:?}", &cuda_vec[..4]);
            assert!(
                cosine_sim > 0.99,
                "Batch item {i}: cosine sim = {cosine_sim}"
            );
        }
    }

    /// Test a realistic-sized matmul matching MiniLM dimensions (256 tokens, 384 hidden)
    #[test]
    fn cuda_vs_cpu_matmul_large() {
        let cuda = match try_cuda() {
            Some(b) => b,
            None => {
                eprintln!("CUDA not available, skipping");
                return;
            }
        };
        let cpu = CpuBackend;

        // A(256, 384) * B(384, 384) = C(256, 384) — typical linear projection
        let a_data: Vec<f32> = (0..256 * 384)
            .map(|i| ((i as f32 * 0.0001).sin()))
            .collect();
        let b_data: Vec<f32> = (0..384 * 384)
            .map(|i| ((i as f32 * 0.00013).cos() * 0.02))
            .collect();

        let a = Tensor::from_slice(&a_data, 256, 384);
        let b = Tensor::from_slice(&b_data, 384, 384);

        let c_cpu = cpu.download(&cpu.matmul(&cpu.upload(&a), &cpu.upload(&b)));
        let c_cuda = cuda.download(&cuda.matmul(&cuda.upload(&a), &cuda.upload(&b)));

        let diff = max_abs_diff(&c_cpu.data, &c_cuda.data);
        eprintln!("matmul_large max diff: {diff}");
        eprintln!("CPU first 8: {:?}", &c_cpu.data[..8]);
        eprintln!("CUDA first 8: {:?}", &c_cuda.data[..8]);
        assert!(diff < 1e-2, "matmul_large: max abs diff = {diff}");
    }
}
