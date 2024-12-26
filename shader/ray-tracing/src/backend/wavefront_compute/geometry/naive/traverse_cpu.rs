#![allow(unused)]

use rendiation_algebra::Vec3;

use crate::backend::wavefront_compute::geometry::intersect_ray_triangle_cpu;
use crate::backend::wavefront_compute::geometry::naive::*;

pub(super) struct NaiveSahBvhCpu {
  // maps user tlas_id to tlas_bvh root node idx in tlas_bvh_forest
  pub(super) tlas_bvh_root: Vec<u32>,
  // global bvh, root at tlas_bvh_root[tlas_idx], content_range to index tlas_data/tlas_bounding
  pub(super) tlas_bvh_forest: Vec<DeviceBVHNode>,
  // acceleration_structure_handle to index blas_meta_info
  pub(super) tlas_data: Vec<TopLevelAccelerationStructureSourceDeviceInstance>,
  pub(super) tlas_bounding: Vec<TlasBounding>,

  pub(super) blas_data: Arc<BuiltBlasPack>,
}

use std::sync::atomic::AtomicU32;
pub(super) static TRI_VISIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static TRI_HIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static TLAS_VISIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static TLAS_HIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static BLAS_VISIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static BLAS_HIT_COUNT: AtomicU32 = AtomicU32::new(0);

impl NaiveSahBvhCpu {
  pub(super) fn traverse(
    &self,
    ray: &ShaderRayTraceCallStoragePayload,
    any_hit: &mut impl FnMut(HitPoint) -> RayAnyHitBehavior,
  ) {
    let flags = TraverseFlags::from_ray_flag(ray.ray_flags);
    let ray_range = RayRangeCpu::new(ray.range.x, ray.range.y, 1.);

    let tlas_bvh_root = self.tlas_bvh_root[ray.tlas_idx as usize];

    // traverse tlas bvh, hit leaf
    let tlas_iter = TraverseBvhIteratorCpu {
      bvh: &self.tlas_bvh_forest,
      ray_origin: ray.ray_origin,
      ray_direction: ray.ray_direction,
      ray_range: ray_range.clone(),
      curr_idx: tlas_bvh_root,
    };
    'tlas_loop: for hit_idx in tlas_iter {
      let node = &self.tlas_bvh_forest[hit_idx as usize];

      // for each tlas, visit blas
      for tlas_idx in node.content_range.x..node.content_range.y {
        // test tlas bounding
        let tlas_bounding = &self.tlas_bounding[tlas_idx as usize];
        if !intersect_ray_aabb_cpu(
          ray.ray_origin,
          ray.ray_direction,
          ray.range,
          tlas_bounding.world_min,
          tlas_bounding.world_max,
        ) {
          continue;
        }
        if ray.cull_mask & tlas_bounding.mask == 0 {
          continue;
        }

        let tlas_data = &self.tlas_data[tlas_idx as usize];
        let flags = TraverseFlags::merge_geometry_instance_flag(flags, tlas_data.flags);

        // hit tlas
        let blas_idx = tlas_data.acceleration_structure_handle;
        // traverse blas bvh
        let blas_ray_origin = tlas_data.transform_inv * ray.ray_origin.expand_with_one();
        let blas_ray_origin = blas_ray_origin.xyz() / blas_ray_origin.w();
        let blas_ray_direction = tlas_data.transform_inv.to_mat3() * ray.ray_direction;
        let distance_scaling = blas_ray_direction.length();
        let blas_ray_range = ray_range.clone_with_scaling(distance_scaling);
        let blas_ray_direction = blas_ray_direction.normalize();

        if flags.visit_triangles() {
          let end_search = self.blas_data.intersect_blas_cpu(
            blas_idx,
            blas_ray_origin,
            blas_ray_direction,
            blas_ray_range,
            distance_scaling,
            flags,
            any_hit,
          );
          if end_search {
            break 'tlas_loop;
          }
        }
      }
    }
  }
}

use std::cell::Cell;
use std::rc::Rc;
#[derive(Clone)]
pub(crate) struct RayRangeCpu {
  near: f32,
  far: Rc<Cell<f32>>,
  scaling: f32,
}
impl RayRangeCpu {
  pub fn new(near: f32, far: f32, scaling: f32) -> Self {
    Self {
      near,
      far: Rc::new(Cell::new(far)),
      scaling,
    }
  }
  pub fn clone_with_scaling(&self, scaling: f32) -> Self {
    Self {
      near: self.near,
      far: self.far.clone(),
      scaling,
    }
  }

  pub fn update_far(&self, far: f32) {
    assert!(self.near <= far);
    assert!(far <= self.far.get());
    self.far.set(far);
  }
  pub fn get(&self) -> Vec2<f32> {
    let far = self.far.get();
    Vec2::new(self.near * self.scaling, far * self.scaling)
  }
}

struct TraverseBvhIteratorCpu<'a> {
  bvh: &'a [DeviceBVHNode],
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: RayRangeCpu,

  curr_idx: u32,
}
impl<'a> Iterator for TraverseBvhIteratorCpu<'a> {
  type Item = u32;
  fn next(&mut self) -> Option<Self::Item> {
    while self.curr_idx != INVALID_NEXT {
      TLAS_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      let node = &self.bvh[self.curr_idx as usize];
      if intersect_ray_aabb_cpu(
        self.ray_origin,
        self.ray_direction,
        self.ray_range.get(),
        node.aabb_min,
        node.aabb_max,
      ) {
        let curr = self.curr_idx;
        self.curr_idx = node.hit_next;

        if node.hit_next == node.miss_next {
          // leaf node
          TLAS_HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
          return Some(curr);
        }
      } else {
        self.curr_idx = node.miss_next;
      };
    }

    None
  }
}

/// general bvh traversal, return hit idx (before redirection)
struct TraverseBvhIteratorCpu2<'a> {
  bvh: &'a [DeviceBVHNode],
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: RayRangeCpu,
  bvh_offset: u32,

  curr_idx: u32,
}
impl<'a> Iterator for TraverseBvhIteratorCpu2<'a> {
  type Item = Vec2<u32>;
  fn next(&mut self) -> Option<Vec2<u32>> {
    while self.curr_idx != INVALID_NEXT {
      BLAS_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      let node = &self.bvh[(self.curr_idx + self.bvh_offset) as usize];
      if intersect_ray_aabb_cpu(
        self.ray_origin,
        self.ray_direction,
        self.ray_range.get(),
        node.aabb_min,
        node.aabb_max,
      ) {
        self.curr_idx = node.hit_next;

        if node.hit_next == node.miss_next {
          // leaf node
          BLAS_HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
          return Some(node.content_range);
        }
      } else {
        self.curr_idx = node.miss_next;
      };
    }

    None
  }
}

impl BuiltBlasPack {
  /// returns end_search
  pub fn intersect_blas_cpu(
    &self,
    blas_idx: u32,
    ray_origin: Vec3<f32>,
    ray_direction: Vec3<f32>,
    ray_range: RayRangeCpu,
    distance_scaling: f32,
    flags: TraverseFlags,
    on_hit: &mut impl FnMut(HitPoint) -> RayAnyHitBehavior,
  ) -> bool {
    let blas = &self.blas_meta[blas_idx as usize];

    for geometry_idx in 0..blas.geometry_count {
      let index = (blas.geometry_offset + geometry_idx) as usize;
      let meta = self.geometry_meta[index];

      assert_eq!(meta.geometry_idx, geometry_idx);

      let bvh_root = meta.bvh_root + blas.bvh_offset;
      let geometry_flags = meta.geometry_flags;
      let geometry_idx = meta.geometry_idx;
      let primitives_offset = meta.primitives_offset + blas.primitives_offset;
      let vertices_offset = meta.vertices_offset + blas.vertices_offset;

      let (pass, is_opaque) = TraverseFlags::cull_geometry(flags, geometry_flags);
      if !pass {
        continue;
      }
      let (cull_enable, cull_back) = TraverseFlags::cull_triangle(flags);

      let iter_bvh = TraverseBvhIteratorCpu2 {
        bvh: &self.bvh,
        ray_origin,
        ray_direction,
        ray_range: ray_range.clone(),
        bvh_offset: bvh_root, // root is offset
        curr_idx: 0,          // start from first local node
      };
      for content_range in iter_bvh {
        let start = content_range.x + primitives_offset;
        let end = content_range.y + primitives_offset;

        for primitive_idx in start..end {
          let primitive_idx_local = self.primitive_redirect[primitive_idx as usize];
          let primitive_idx = (primitive_idx_local + primitives_offset) as usize;
          let a = self.indices[primitive_idx * 3] + vertices_offset;
          let b = self.indices[primitive_idx * 3 + 1] + vertices_offset;
          let c = self.indices[primitive_idx * 3 + 2] + vertices_offset;
          let a = self.vertices[a as usize];
          let b = self.vertices[b as usize];
          let c = self.vertices[c as usize];

          TRI_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

          // (hit, distance, u, v)
          let intersection = intersect_ray_triangle_cpu(
            ray_origin,
            ray_direction,
            ray_range.get(),
            a,
            b,
            c,
            cull_enable,
            cull_back,
          );

          if intersection[0] != 0. {
            TRI_HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let distance = intersection[1] / distance_scaling;
            let position = ray_origin + distance * ray_direction;

            let hit_point = HitPoint {
              geometry_idx,
              primitive_idx: primitive_idx_local,
              distance,
              position,
              uv: vec2(intersection[2], intersection[3]),
              is_opaque,
            };
            let mut behavior = on_hit(hit_point);

            if behavior & ANYHIT_BEHAVIOR_ACCEPT_HIT > 0 {
              ray_range.update_far(distance);

              if flags.end_search_on_hit() {
                behavior |= ANYHIT_BEHAVIOR_END_SEARCH;
              }
            }
            if behavior & ANYHIT_BEHAVIOR_END_SEARCH > 0 {
              return true;
            }
          }
        }
      }
    }
    false
  }
}
