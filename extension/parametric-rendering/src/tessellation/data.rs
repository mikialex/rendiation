//! Static model data and per frame inputs of the GPU tessellation (plan M1).

use std::num::NonZeroU64;

use parametric_normalized_model::{FaceInstance, ParametricNormalizedModel};
use parametric_tessellation::*;

use super::*;
use crate::*;

pub const ITEM_FLAG_FLIP_NORMAL: u32 = 1;
pub const ITEM_FLAG_TRIMMED: u32 = 2;

/// One (instance, patch) occurrence. Its camera space control points are at
/// `camera_cp_offset..`, `v * (degree_u + 1) + u` order like `RationalBezierSurface`.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TessItem {
  pub instance: u32,
  pub patch: u32,
  pub camera_cp_offset: u32,
  pub degree_u: u32,
  pub degree_v: u32,
  /// `ITEM_FLAG_*` bits.
  pub flags: u32,
}

/// Values shared by all passes of a frame.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Debug, Clone, Copy)]
pub struct TessUniform {
  pub focal_length: f32,
  pub near: f32,
  /// Half the viewport over the focal length (frustum side slopes).
  pub frustum_x: f32,
  pub frustum_y: f32,
  /// Camera space tolerances are these times depth over focal length.
  pub interior_tolerance: f32,
  pub edge_tolerance: f32,
  pub max_triangle_size: f32,
  pub k_max: u32,
  pub log2_k_max: u32,
  pub max_level: u32,
  /// Vertex position rounds after round 0, `max_level + log2_k_max`.
  pub rounds: u32,
  pub item_count: u32,
  pub camera_cp_count: u32,
  pub sub_patch_stride: u32,
  pub queue_capacity: u32,
  pub leaf_capacity: u32,
  pub vertex_capacity: u32,
  pub request_capacity: u32,
}

impl TessUniform {
  /// Tolerances are computed in f64 like the CPU reference, then rounded.
  pub fn new(camera: &Camera, config: &TessellationConfig) -> Self {
    let log2_k_max = config.k_max.trailing_zeros();
    Self {
      focal_length: camera.focal_length as f32,
      near: camera.near as f32,
      frustum_x: (camera.viewport.x * 0.5 / camera.focal_length) as f32,
      frustum_y: (camera.viewport.y * 0.5 / camera.focal_length) as f32,
      interior_tolerance: (config.pixel_error * (1. - config.edge_error_share)) as f32,
      edge_tolerance: (config.pixel_error * config.edge_error_share) as f32,
      max_triangle_size: config.max_triangle_size as f32,
      k_max: config.k_max,
      log2_k_max,
      max_level: config.max_level,
      rounds: config.max_level + log2_k_max,
      ..Zeroable::zeroed()
    }
  }

  pub fn with_sizes(self, data: &TessStaticData, capacity: &TessCapacity) -> Self {
    Self {
      item_count: data.items.len() as u32,
      camera_cp_count: data.cp_sources.len() as u32,
      sub_patch_stride: data.sub_patch_stride,
      queue_capacity: capacity.queue,
      leaf_capacity: capacity.leaves,
      vertex_capacity: capacity.vertices,
      request_capacity: capacity.requests,
      ..self
    }
  }
}

/// Capacities of the per frame buffers, in elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TessCapacity {
  /// Work items per level (each of the two ping-pong queues).
  pub queue: u32,
  pub leaves: u32,
  pub vertices: u32,
  /// Position requests of all rounds after round 0.
  pub requests: u32,
}

/// Indirect dispatches are limited to 65535 workgroups per dimension, the 1D passes use
/// workgroups of `SUBDIVISION_WORKGROUP` or `VERTEX_WORKGROUP` threads.
pub const SUBDIVISION_WORKGROUP: u32 = 64;
pub const VERTEX_WORKGROUP: u32 = 256;
pub const MAX_DISPATCH_WORKGROUPS: u32 = 65535;

impl TessCapacity {
  pub fn initial(item_count: usize) -> Self {
    let items = item_count.max(1) as u32;
    Self {
      queue: (items * 2).max(4096),
      leaves: (items * 2).max(4096),
      vertices: (items * 64).max(1 << 18),
      requests: (items * 64).max(1 << 18),
    }
    .clamped(item_count)
  }

  /// Within `max_binding` bytes per buffer (the sub patches, vertices and the request
  /// records are the largest).
  pub fn limited(self, max_binding: u64, sub_patch_stride: u32) -> Self {
    let elements =
      |bytes_per_element: u64| (max_binding / bytes_per_element).min(u32::MAX as u64) as u32;
    let records = elements(16);
    let vertices = self
      .vertices
      .min(elements(size_of::<TessVertex>() as u64))
      .min(records / 2);
    Self {
      queue: self.queue.min(elements(16 * sub_patch_stride as u64)) & !3,
      leaves: self.leaves,
      vertices,
      requests: self.requests.min(records - vertices) & !1,
    }
  }

  /// Within the indirect dispatch limits, the queue holds at least all roots.
  pub fn clamped(self, item_count: usize) -> Self {
    let subdivision = SUBDIVISION_WORKGROUP * MAX_DISPATCH_WORKGROUPS;
    let vertex = VERTEX_WORKGROUP * MAX_DISPATCH_WORKGROUPS;
    Self {
      queue: self
        .queue
        .max(item_count as u32)
        .min(subdivision)
        .next_multiple_of(4),
      leaves: self.leaves.min(subdivision),
      vertices: self.vertices.min(vertex),
      requests: self.requests.min(vertex) & !1,
    }
  }
}

/// The CPU side of the static upload: face origin relative f32 control points of every patch
/// once, and the item table of the (instance, patch) occurrences.
#[derive(Debug, Clone)]
pub struct TessStaticData {
  pub items: Vec<TessItem>,
  /// Face origin relative control points of all accepted patches, tightly packed.
  pub cp_pool: Vec<Vec4<f32>>,
  /// Per model patch, its offset in `cp_pool` (`u32::MAX` if rejected for its degree).
  pub patch_cp_offset: Vec<u32>,
  /// Per camera space control point, its `cp_pool` index and instance.
  pub cp_sources: Vec<Vec2<u32>>,
  pub face_origins: Vec<Vec3<f64>>,
  pub instances: Vec<FaceInstance>,
  /// Patches (not occurrences) rejected for a degree above `MAX_GPU_DEGREE`.
  pub rejected_patches: usize,
  /// Sub patch buffer slot size, the largest control point count of the items.
  pub sub_patch_stride: u32,
}

impl TessStaticData {
  /// The items are in the same order as `camera_space_items` with the same filter, so item
  /// indices match the CPU reference.
  pub fn new(model: &ParametricNormalizedModel, untrimmed_only: bool) -> Self {
    let face_origins = face_origins(model);
    let mut cp_pool = Vec::new();
    let mut rejected_patches = 0;
    let patch_cp_offset: Vec<_> = model
      .patches
      .iter()
      .map(|patch| {
        let s = &patch.surface;
        if s.degree_u > MAX_GPU_DEGREE || s.degree_v > MAX_GPU_DEGREE {
          rejected_patches += 1;
          return u32::MAX;
        }
        let offset = cp_pool.len() as u32;
        let relative = relative_to_origin(s, face_origins[patch.face.index()]);
        cp_pool.extend(relative.control_points.iter().map(|p| p.into_f32()));
        offset
      })
      .collect();

    let mut items = Vec::new();
    let mut cp_sources = Vec::new();
    let mut sub_patch_stride = 1;
    for (instance_index, instance) in model.instances.iter().enumerate() {
      for patch_index in instance.patches.clone() {
        let patch = &model.patches[patch_index];
        let offset = patch_cp_offset[patch_index];
        if (untrimmed_only && !patch.is_untrimmed()) || offset == u32::MAX {
          continue;
        }
        let (n, m) = (patch.surface.degree_u as u32, patch.surface.degree_v as u32);
        let count = (n + 1) * (m + 1);
        sub_patch_stride = sub_patch_stride.max(count);
        let mut flags = 0;
        if patch.flip_normal != instance.flip_normal {
          flags |= ITEM_FLAG_FLIP_NORMAL;
        }
        if !patch.is_untrimmed() {
          flags |= ITEM_FLAG_TRIMMED;
        }
        items.push(TessItem {
          instance: instance_index as u32,
          patch: patch_index as u32,
          camera_cp_offset: cp_sources.len() as u32,
          degree_u: n,
          degree_v: m,
          flags,
          ..Default::default()
        });
        cp_sources.extend((0..count).map(|i| Vec2::new(offset + i, instance_index as u32)));
      }
    }

    Self {
      items,
      cp_pool,
      patch_cp_offset,
      cp_sources,
      face_origins,
      instances: model.instances.clone(),
      rejected_patches,
      sub_patch_stride,
    }
  }

  /// The same data without the trimmed items (the item order is kept).
  pub fn untrimmed(&self) -> Self {
    let mut cp_sources = Vec::new();
    let items = self
      .items
      .iter()
      .filter(|item| item.flags & ITEM_FLAG_TRIMMED == 0)
      .map(|item| {
        let count = (item.degree_u + 1) * (item.degree_v + 1);
        let offset = self.patch_cp_offset[item.patch as usize];
        let camera_cp_offset = cp_sources.len() as u32;
        cp_sources.extend((0..count).map(|i| Vec2::new(offset + i, item.instance)));
        TessItem {
          camera_cp_offset,
          ..*item
        }
      })
      .collect();
    Self {
      items,
      cp_sources,
      ..self.clone()
    }
  }

  /// Per instance, `item_to_camera` composed in f64 then rounded to f32.
  pub fn instance_matrices(&self, camera: &Camera) -> Vec<Mat4<f32>> {
    self
      .instances
      .iter()
      .map(|instance| {
        item_to_camera(camera, instance, self.face_origins[instance.face.index()]).into_f32()
      })
      .collect()
  }
}

/// The static data on the GPU plus the per frame instance matrices.
pub struct TessDeviceData {
  pub items: StorageBufferReadonlyDataView<[TessItem]>,
  pub cp_pool: StorageBufferReadonlyDataView<[Vec4<f32>]>,
  pub cp_sources: StorageBufferReadonlyDataView<[Vec2<u32>]>,
  pub matrices: StorageBufferReadonlyDataView<[Mat4<f32>]>,
  /// Written by the transform pass.
  pub camera_cps: StorageBufferDataView<[Vec4<f32>]>,
}

impl TessDeviceData {
  pub fn new(device: &GPUDevice, data: &TessStaticData) -> Self {
    let dispatch_limit = |workgroup: u32| (workgroup * MAX_DISPATCH_WORKGROUPS) as usize;
    assert!(
      data.items.len() <= dispatch_limit(SUBDIVISION_WORKGROUP),
      "too many items"
    );
    assert!(
      data.cp_sources.len() <= dispatch_limit(VERTEX_WORKGROUP),
      "too many control points"
    );
    let instance_count = data.instances.len();
    let non_empty = |len: usize| len.max(1);
    let items = if data.items.is_empty() {
      vec![TessItem::default()]
    } else {
      data.items.clone()
    };
    let cp_pool = if data.cp_pool.is_empty() {
      vec![Vec4::zero()]
    } else {
      data.cp_pool.clone()
    };
    let cp_sources = if data.cp_sources.is_empty() {
      vec![Vec2::zero()]
    } else {
      data.cp_sources.clone()
    };
    Self {
      items: create_gpu_readonly_storage(items.as_slice(), device, "tessellation items"),
      cp_pool: create_gpu_readonly_storage(cp_pool.as_slice(), device, "tessellation cp pool"),
      cp_sources: create_gpu_readonly_storage(
        cp_sources.as_slice(),
        device,
        "tessellation cp sources",
      ),
      matrices: StorageBufferReadonlyDataView::create_by(
        device,
        "tessellation instance matrices",
        StorageBufferInit::Zeroed(
          NonZeroU64::new((non_empty(instance_count) * size_of::<Mat4<f32>>()) as u64).unwrap(),
        ),
      ),
      camera_cps: create_gpu_read_write_storage::<[Vec4<f32>]>(
        ZeroedArrayByArrayLength(non_empty(data.cp_sources.len())),
        device,
        "tessellation camera cps",
      ),
    }
  }

  pub fn write_matrices(&self, queue: &GPUQueue, matrices: &[Mat4<f32>]) {
    if !matrices.is_empty() {
      self.matrices.write_at(0, cast_slice(matrices), queue);
    }
  }
}
