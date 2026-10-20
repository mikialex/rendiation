use parametric_normalized_model::ParametricNormalizedModel;
use parametric_tessellation::{Leaf, Tessellator, param_to_f64};

use crate::*;

/// Leaf grid resolutions `k = 1, 2, 4, 8, 16`, one draw per bucket.
pub const K_BUCKET_COUNT: usize = 5;

/// One leaf grid vertex, camera space.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TessVertex {
  /// `xyz` position, `w` = 1.
  pub position: Vec4<f32>,
  /// `xyz` unit normal facing out of the face (patch and instance flips applied), `w` = 0.
  pub normal: Vec4<f32>,
}

pub const LEAF_FLAG_TRIMMED: u32 = 1;

/// One leaf to draw. Its `(k + 1)²` vertices are at `vertex_base..`, u index fastest.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TessLeafDraw {
  pub vertex_base: u32,
  pub k: u32,
  /// Index into the tessellation item table (one (instance, patch) occurrence).
  pub item: u32,
  /// `LEAF_FLAG_*` bits.
  pub flags: u32,
}

/// Per frame result of the adaptive tessellation, the interface between the producer (GPU
/// compute, or the uploaded CPU reference) and the draw component.
///
/// Drawing: for bucket `b` (`k = 1 << b`) issue one non indexed `draw_indirect` with
/// `draw_args[b]` (`vertex_count = 6·k²`, `instance_count` = leaves in the bucket). The vertex
/// shader takes leaf `leaves[bucket_leaves[b * bucket_capacity + instance_index]]`, cell
/// `vertex_index / 6` (u fastest) and the corner `vertex_index % 6` of the two triangles
/// `(v00, v10, v11), (v00, v11, v01)`, counter clockwise in the parameter domain.
#[derive(Clone)]
pub struct TessellationFrameOutput {
  pub vertices: StorageBufferDataView<[TessVertex]>,
  pub leaves: StorageBufferDataView<[TessLeafDraw]>,
  pub bucket_leaves: StorageBufferDataView<[u32]>,
  pub bucket_capacity: u32,
  /// `K_BUCKET_COUNT` entries, the buffer also has `INDIRECT` usage.
  pub draw_args: StorageBufferDataView<[DrawIndirectArgsStorage]>,
}

impl TessellationFrameOutput {
  /// Zeroed buffers (all draws empty).
  pub fn new(device: &GPUDevice, vertex_capacity: u32, leaf_capacity: u32) -> Self {
    Self {
      vertices: create_gpu_read_write_storage::<[TessVertex]>(
        ZeroedArrayByArrayLength(vertex_capacity.max(1) as usize),
        device,
        "tessellation vertices",
      ),
      leaves: create_gpu_read_write_storage::<[TessLeafDraw]>(
        ZeroedArrayByArrayLength(leaf_capacity.max(1) as usize),
        device,
        "tessellation leaves",
      ),
      bucket_leaves: create_gpu_read_write_storage::<[u32]>(
        ZeroedArrayByArrayLength(leaf_capacity.max(1) as usize * K_BUCKET_COUNT),
        device,
        "tessellation bucket leaves",
      ),
      bucket_capacity: leaf_capacity.max(1),
      draw_args: create_draw_args_buffer(device, &empty_draw_args()),
    }
  }

  /// Fill the output from the CPU reference: the leaves of `t.run()` with their meshes.
  pub fn from_cpu_reference(
    device: &GPUDevice,
    model: &ParametricNormalizedModel,
    t: &mut Tessellator,
    leaves: &[Leaf],
  ) -> Self {
    let mut vertices = Vec::new();
    let mut draws = Vec::with_capacity(leaves.len());
    let mut buckets = vec![Vec::new(); K_BUCKET_COUNT];
    for leaf in leaves {
      let item = &t.items()[leaf.key.item as usize];
      let patch = &model.patches[item.patch as usize];
      let flip = patch.flip_normal != model.instances[item.instance as usize].flip_normal;
      let surface = item.surface.clone();
      let mesh = t.leaf_mesh(leaf);
      buckets[leaf.k.trailing_zeros() as usize].push(draws.len() as u32);
      draws.push(TessLeafDraw {
        vertex_base: vertices.len() as u32,
        k: leaf.k,
        item: leaf.key.item,
        flags: if patch.is_untrimmed() {
          0
        } else {
          LEAF_FLAG_TRIMMED
        },
        ..Default::default()
      });
      for (&(u, v), p) in mesh.params.iter().zip(&mesh.positions) {
        let (_, du, dv) = surface.eval_with_partials(param_to_f64(u), param_to_f64(v));
        let n = du.cross(dv);
        let n = if n.length() > 0. { n.normalize() } else { n };
        let n = if flip { n * -1. } else { n };
        vertices.push(TessVertex {
          position: Vec4::new(p.x, p.y, p.z, 1.).into_f32(),
          normal: Vec4::new(n.x, n.y, n.z, 0.).into_f32(),
          ..Default::default()
        });
      }
    }

    let bucket_capacity = buckets.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut bucket_leaves = vec![0; bucket_capacity * K_BUCKET_COUNT];
    let mut args = empty_draw_args();
    for (b, bucket) in buckets.iter().enumerate() {
      bucket_leaves[b * bucket_capacity..][..bucket.len()].copy_from_slice(bucket);
      args[b].instance_count = bucket.len() as u32;
    }
    if vertices.is_empty() {
      vertices.push(TessVertex::default());
    }
    if draws.is_empty() {
      draws.push(TessLeafDraw::default());
    }
    Self {
      vertices: create_gpu_read_write_storage(vertices.as_slice(), device, "tessellation vertices"),
      leaves: create_gpu_read_write_storage(draws.as_slice(), device, "tessellation leaves"),
      bucket_leaves: create_gpu_read_write_storage(
        bucket_leaves.as_slice(),
        device,
        "tessellation bucket leaves",
      ),
      bucket_capacity: bucket_capacity as u32,
      draw_args: create_draw_args_buffer(device, &args),
    }
  }
}

fn empty_draw_args() -> [DrawIndirectArgsStorage; K_BUCKET_COUNT] {
  std::array::from_fn(|b| {
    let k = 1u32 << b;
    DrawIndirectArgsStorage::new(6 * k * k, 0, 0, 0)
  })
}

fn create_draw_args_buffer(
  device: &GPUDevice,
  args: &[DrawIndirectArgsStorage],
) -> StorageBufferDataView<[DrawIndirectArgsStorage]> {
  let usage = BufferUsages::INDIRECT
    | BufferUsages::STORAGE
    | BufferUsages::COPY_DST
    | BufferUsages::COPY_SRC;
  let buffer = create_gpu_buffer(cast_slice(args), usage, device).create_default_view();
  StorageBufferDataView::try_from_raw(buffer).unwrap()
}
