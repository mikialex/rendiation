use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use parametric_normalized_model::ParametricNormalizedModel;
use parametric_tessellation::*;

use super::*;
use crate::*;

/// The GPU view dependent adaptive tessellation: uploads the model once, then each frame
/// records the compute passes producing a `TessellationFrameOutput` for the camera.
///
/// Statistics and overflow flags are read back asynchronously from an earlier frame. When a
/// frame overflowed a buffer, the buffers are grown before the next frame is encoded (the
/// overflowing frame drops the writes that did not fit, so it may miss geometry, and the
/// output buffers are replaced by larger ones).
pub struct GpuTessellator {
  device: GPUDevice,
  config: TessellationConfig,
  untrimmed_only: bool,
  /// The active items changed, the device data is uploaded again at the next frame.
  upload: bool,
  all: TessStaticData,
  pub(crate) active: TessStaticData,
  pub(crate) device_data: TessDeviceData,
  capacity: TessCapacity,
  pub(crate) frame: TessFrameBuffers,
  stats: Option<GpuTessellationStats>,
  pending_stats: Option<(ReadBufferFromStagingBuffer, TessCapacity)>,
}

impl GpuTessellator {
  /// Static upload of all (instance, patch) items of the model (face origin relative f32
  /// control points). Patches above `MAX_GPU_DEGREE` are rejected and counted.
  pub fn new(gpu: &GPU, model: &ParametricNormalizedModel, config: TessellationConfig) -> Self {
    check_config(&config);
    let all = TessStaticData::new(model, false);
    let active = all.clone();
    let device_data = TessDeviceData::new(&gpu.device, &active);
    let capacity = TessCapacity::initial(active.items.len());
    let capacity = device_limited(&gpu.device, capacity, active.sub_patch_stride);
    let frame = TessFrameBuffers::new(&gpu.device, capacity, &config, active.sub_patch_stride);
    Self {
      device: gpu.device.clone(),
      config,
      untrimmed_only: false,
      upload: false,
      all,
      active,
      device_data,
      capacity,
      frame,
      stats: None,
      pending_stats: None,
    }
  }

  pub fn set_config(&mut self, config: TessellationConfig) {
    check_config(&config);
    let levels_changed =
      config.max_level != self.config.max_level || config.k_max != self.config.k_max;
    self.config = config;
    if levels_changed {
      self.rebuild_frame_buffers(self.capacity);
    }
  }

  pub fn config(&self) -> &TessellationConfig {
    &self.config
  }

  /// Only tessellate the untrimmed patches (like the CPU `camera_space_items` filter), the
  /// item indices then match `camera_space_items(model, camera, |p| p.is_untrimmed())`.
  pub fn set_untrimmed_only(&mut self, untrimmed_only: bool) {
    if untrimmed_only == self.untrimmed_only {
      return;
    }
    self.untrimmed_only = untrimmed_only;
    self.active = if untrimmed_only {
      self.all.untrimmed()
    } else {
      self.all.clone()
    };
    self.upload = true;
  }

  /// Records all compute passes of one frame into `encoder`, the returned output is valid to
  /// draw after the encoder is submitted (by `GPU::submit_encoder`, which also completes the
  /// asynchronous statistics read). The output buffers may be replaced between frames.
  pub fn encode_frame(
    &mut self,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    camera: &Camera,
  ) -> &TessellationFrameOutput {
    if self.upload {
      self.upload = false;
      self.device_data = TessDeviceData::new(&gpu.device, &self.active);
      self.pending_stats = None;
      self.rebuild_frame_buffers(TessCapacity::initial(self.active.items.len()));
    }
    self.poll_stats(gpu);

    let uniform = TessUniform::new(camera, &self.config).with_sizes(&self.active, &self.capacity);
    self.frame.uniform.write_at(&gpu.queue, &uniform, 0);
    let matrices = self.active.instance_matrices(camera);
    self.device_data.write_matrices(&gpu.queue, &matrices);
    self.frame.encode(&gpu.device, encoder, &self.device_data);
    if self.pending_stats.is_none() {
      let read = encoder.read_buffer(&gpu.device, &self.frame.counters.gpu);
      self.pending_stats = Some((read, self.capacity));
    }
    &self.frame.output
  }

  /// Statistics and overflow flags read back asynchronously from an earlier frame (`None`
  /// until available).
  pub fn last_stats(&self) -> Option<&GpuTessellationStats> {
    self.stats.as_ref()
  }

  /// The output of the last encoded frame.
  pub fn output(&self) -> &TessellationFrameOutput {
    &self.frame.output
  }

  /// The item table, `TessLeafDraw::item` indexes it.
  pub fn items(&self) -> &[TessItem] {
    &self.active.items
  }

  pub fn capacity(&self) -> TessCapacity {
    self.capacity
  }

  /// Replaces the capacities (for example to test the overflow handling), the buffers are
  /// recreated.
  pub fn set_capacity(&mut self, capacity: TessCapacity) {
    self.rebuild_frame_buffers(capacity.clamped(self.active.items.len()));
  }

  /// Compute dispatches recorded per frame.
  pub fn dispatch_count(&self) -> u32 {
    self.frame.dispatch_count()
  }

  /// Patches rejected at upload for a degree above `MAX_GPU_DEGREE`.
  pub fn rejected_patches(&self) -> usize {
    self.all.rejected_patches
  }

  /// Blocking read of the buffers of the last encoded frame, which must have been submitted.
  /// For tests and tools.
  pub async fn read_back(&self, gpu: &GPU) -> GpuFrameReadback {
    let mut encoder = gpu.create_encoder();
    let frame = &self.frame;
    let counters = encoder.read_buffer_bytes(&gpu.device, &frame.counters.gpu);
    let leaves = encoder.read_storage_array(&gpu.device, &frame.output.leaves);
    let keys = encoder.read_storage_array(&gpu.device, &frame.leaf_keys);
    let vertices = encoder.read_storage_array(&gpu.device, &frame.output.vertices);
    let draw_args = encoder.read_storage_array(&gpu.device, &frame.output.draw_args);
    gpu.submit_encoder(encoder);
    let counters: Vec<u32> = cast_slice(&counters.await.unwrap()).to_vec();
    let leaf_count =
      (counters[COUNTER_LEAVES as usize] as usize).min(self.capacity.leaves as usize);
    let mut leaves = leaves.await.unwrap();
    let mut keys = keys.await.unwrap();
    leaves.truncate(leaf_count);
    keys.truncate(leaf_count);
    GpuFrameReadback {
      stats: GpuTessellationStats::from_counters(&counters, frame, self),
      leaves,
      keys,
      vertices: vertices.await.unwrap(),
      draw_args: draw_args.await.unwrap(),
    }
  }

  /// Polls the pending statistics read without blocking, grows the buffers on overflow.
  fn poll_stats(&mut self, gpu: &GPU) {
    let Some((pending, capacity)) = &mut self.pending_stats else {
      return;
    };
    gpu.poll(false);
    let mut cx = Context::from_waker(Waker::noop());
    let Poll::Ready(result) = Pin::new(pending).poll(&mut cx) else {
      return;
    };
    let capacity = *capacity;
    self.pending_stats = None;
    let Ok(buffer) = result else {
      return;
    };
    let counters: Vec<u32> = cast_slice(&buffer.read_raw()).to_vec();
    drop(buffer);
    let stats = GpuTessellationStats::from_counters(&counters, &self.frame, self);
    // A read from before the last growth may report overflows already handled.
    if stats.overflow != 0 && capacity == self.capacity {
      self.rebuild_frame_buffers(stats.grown_capacity(capacity));
    }
    self.stats = Some(stats);
  }

  fn rebuild_frame_buffers(&mut self, capacity: TessCapacity) {
    let stride = self.active.sub_patch_stride;
    let capacity = device_limited(&self.device, capacity, stride);
    self.capacity = capacity;
    self.frame = TessFrameBuffers::new(&self.device, capacity, &self.config, stride);
  }
}

fn device_limited(device: &GPUDevice, capacity: TessCapacity, stride: u32) -> TessCapacity {
  let limits = device.limits();
  let max_binding = limits
    .max_storage_buffer_binding_size
    .min(limits.max_buffer_size);
  capacity.limited(max_binding, stride)
}

fn check_config(config: &TessellationConfig) {
  assert!(config.k_max.is_power_of_two());
  assert!(
    (config.k_max.trailing_zeros() as usize) < K_BUCKET_COUNT,
    "k_max above the draw buckets"
  );
  assert!(config.max_level + config.k_max.trailing_zeros() <= PARAM_BITS);
}

/// Statistics of one frame from the GPU counters.
#[derive(Debug, Clone, Default)]
pub struct GpuTessellationStats {
  pub items: usize,
  pub rejected_patches: usize,
  pub leaves: u32,
  pub triangles: u64,
  /// Allocated leaf vertices (requested, may exceed the capacity on overflow).
  pub vertices: u32,
  /// Work items per level (requested, may exceed the capacity on overflow).
  pub work_items: Vec<u32>,
  pub leaves_per_level: Vec<u32>,
  /// Leaf count by `log2 k`.
  pub leaves_per_k: [u32; K_BUCKET_COUNT],
  pub culled: u32,
  /// Indexed by `SplitReason as usize`.
  pub splits: [u32; 4],
  pub forced_leaves: u32,
  pub edge_raised_leaves: u32,
  /// Position requests per round, round 0 are the leaf boundary vertices (not counted, 0).
  pub requests_per_round: Vec<u32>,
  /// `OVERFLOW_*` bits.
  pub overflow: u32,
  pub capacity: Option<TessCapacity>,
  pub dispatches: u32,
}

impl GpuTessellationStats {
  fn from_counters(c: &[u32], frame: &TessFrameBuffers, t: &GpuTessellator) -> Self {
    let at = |i: u32| c[i as usize];
    let range = |base: u32, len: u32| (0..len).map(|i| at(base + i)).collect::<Vec<_>>();
    let leaves_per_k = std::array::from_fn(|b| at(COUNTER_BUCKETS + b as u32));
    let triangles = leaves_per_k
      .iter()
      .enumerate()
      .map(|(b, &n)| (2 * n as u64) << (2 * b))
      .sum();
    let mut work_items = range(COUNTER_WORK_ITEMS, frame.levels);
    while work_items.last() == Some(&0) {
      work_items.pop();
    }
    let leaves_per_level = range(COUNTER_LEVEL_LEAVES, work_items.len() as u32);
    let mut requests_per_round = range(COUNTER_ROUNDS, frame.rounds + 1);
    while requests_per_round.len() > 1 && requests_per_round.last() == Some(&0) {
      requests_per_round.pop();
    }
    Self {
      items: t.active.items.len(),
      rejected_patches: t.all.rejected_patches,
      leaves: at(COUNTER_LEAVES),
      triangles,
      vertices: at(COUNTER_VERTICES),
      work_items,
      leaves_per_level,
      leaves_per_k,
      culled: at(COUNTER_CULLED),
      splits: std::array::from_fn(|i| at(COUNTER_SPLITS + i as u32)),
      forced_leaves: at(COUNTER_FORCED_LEAVES),
      edge_raised_leaves: at(COUNTER_EDGE_RAISED),
      requests_per_round,
      overflow: at(COUNTER_OVERFLOW),
      capacity: Some(frame.capacity),
      dispatches: frame.dispatch_count(),
    }
  }

  /// Capacities fitting the requested counts with some headroom. Counts behind an overflow
  /// (for example leaves of dropped work items) are underestimated, so overflowing buffers
  /// grow at least twofold and the growth converges over a few frames.
  pub fn grown_capacity(&self, current: TessCapacity) -> TessCapacity {
    let grow = |flag: u32, current: u32, requested: u64| {
      let needed = (requested + requested / 4) as u32;
      if self.overflow & flag != 0 {
        needed.max(current.saturating_mul(2))
      } else {
        current
      }
    };
    let max_work = self.work_items.iter().copied().max().unwrap_or(0) as u64;
    let requests: u64 = self.requests_per_round.iter().map(|&n| n as u64).sum();
    let grown = TessCapacity {
      queue: grow(OVERFLOW_QUEUE, current.queue, max_work),
      leaves: grow(OVERFLOW_LEAVES, current.leaves, self.leaves as u64),
      vertices: grow(OVERFLOW_VERTICES, current.vertices, self.vertices as u64),
      requests: grow(OVERFLOW_REQUESTS, current.requests, requests),
    };
    grown.clamped(self.items)
  }
}

/// Buffers of a frame read back for tests and tools.
pub struct GpuFrameReadback {
  pub stats: GpuTessellationStats,
  pub leaves: Vec<TessLeafDraw>,
  /// `(item, level, ix, iy)` per leaf.
  pub keys: Vec<Vec4<u32>>,
  pub vertices: Vec<TessVertex>,
  pub draw_args: Vec<DrawIndirectArgsStorage>,
}

impl GpuFrameReadback {
  /// The leaves in the CPU reference form.
  pub fn cpu_leaves(&self) -> Vec<Leaf> {
    self
      .leaves
      .iter()
      .zip(&self.keys)
      .map(|(leaf, key)| Leaf {
        key: SubPatchKey {
          item: key.x,
          level: key.y,
          ix: key.z,
          iy: key.w,
        },
        k: leaf.k,
      })
      .collect()
  }

  /// The leaf grids with the GPU positions (exactly converted to f64), in `cpu_leaves` order.
  pub fn leaf_meshes(&self) -> Vec<LeafMesh> {
    self
      .cpu_leaves()
      .iter()
      .zip(&self.leaves)
      .map(|(leaf, draw)| {
        LeafMesh::grid(leaf, |i, j, _, _| {
          let index = draw.vertex_base + j * (leaf.k + 1) + i;
          let p = self.vertices[index as usize].position;
          Vec3::new(p.x, p.y, p.z).into_f64()
        })
      })
      .collect()
  }
}
