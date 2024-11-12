use rendiation_algebra::Vec3;

use crate::backend::wavefront_compute::geometry::intersect_ray_triangle_cpu;
use crate::backend::wavefront_compute::geometry::naive::*;

#[allow(unused)]
#[derive(Debug)]
pub(super) struct NaiveSahBvhCpu {
  // global bvh, root at 0, content_range to index tlas_data/tlas_bounding
  pub(super) tlas_bvh_forest: Vec<DeviceBVHNode>,
  // acceleration_structure_handle to index blas_meta_info
  pub(super) tlas_data: Vec<TopLevelAccelerationStructureSourceDeviceInstance>,
  pub(super) tlas_bounding: Vec<TlasBounding>,

  // tri_range to index tri_bvh_root, box_range to index box_bvh_root
  pub(super) blas_meta_info: Vec<BlasMetaInfo>,
  // tri_bvh_forest root_idx, geometry_idx, primitive_start, geometry_flags
  pub(super) tri_bvh_root: Vec<GeometryMetaInfo>,
  // box_bvh_forest root_idx, geometry_idx, primitive_start, geometry_flags
  pub(super) box_bvh_root: Vec<GeometryMetaInfo>,
  // content range to index indices
  pub(super) tri_bvh_forest: Vec<DeviceBVHNode>,
  // content range to index boxes
  pub(super) box_bvh_forest: Vec<DeviceBVHNode>,

  pub(super) indices: Vec<u32>,
  pub(super) vertices: Vec<Vec3<f32>>,
  pub(super) boxes: Vec<Vec3<f32>>,
}

use std::sync::atomic::AtomicU32;
pub(super) static TRI_VISIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static TRI_HIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static BVH_VISIT_COUNT: AtomicU32 = AtomicU32::new(0);
pub(super) static BVH_HIT_COUNT: AtomicU32 = AtomicU32::new(0);

impl NaiveSahBvhCpu {
  pub(super) fn traverse(
    &self,
    ray: &mut ShaderRayTraceCallStoragePayload,
    any_hit: &mut dyn FnMut(u32, u32, f32, Vec3<f32>) -> bool, /* geometry_idx, primitive_idx, distance, hit_position // todo use ctx */
  ) {
    let flags = TraverseFlags::from_ray_flag_cpu(ray.ray_flags);
    let ray_range = RayRange::new(ray.range.x, ray.range.y, 1.);

    // traverse tlas bvh, hit leaf
    let tlas_iter = TraverseBvhIteratorCpu {
      bvh: &self.tlas_bvh_forest,
      ray_origin: ray.ray_origin,
      ray_direction: ray.ray_direction,
      ray_range: ray_range.clone(),
      curr_idx: 0,
    };
    for hit_idx in tlas_iter {
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
        // hit tlas
        let blas_idx = tlas_data.acceleration_structure_handle;
        let flags = TraverseFlags::apply_geometry_instance_flag_cpu(flags, tlas_data.flags);

        // traverse blas bvh
        let blas_ray_origin = tlas_data.transform_inv * ray.ray_origin.expand_with_one();
        let blas_ray_origin = blas_ray_origin.xyz() / blas_ray_origin.w();
        let blas_ray_direction = tlas_data.transform_inv.to_mat3() * ray.ray_direction;
        let distance_scaling = blas_ray_direction.length();
        let blas_ray_range = ray_range.clone_with_scaling(distance_scaling);
        let blas_ray_direction = blas_ray_direction.normalize();

        let blas_meta_info = &self.blas_meta_info[blas_idx as usize];

        if flags.visit_triangles_cpu() {
          for tri_root_index in blas_meta_info.tri_root_range.x..blas_meta_info.tri_root_range.y {
            let geometry = self.tri_bvh_root[tri_root_index as usize];
            let blas_root_idx = geometry.bvh_root_idx;
            let geometry_idx = geometry.geometry_idx;
            let primitive_start = geometry.primitive_start;
            let geometry_flags = geometry.geometry_flags;

            let (pass, _is_opaque) = TraverseFlags::cull_geometry_cpu(flags, geometry_flags);
            if !pass {
              continue;
            }
            let (cull_enable, cull_back) = TraverseFlags::cull_triangle_cpu(flags);

            let bvh_iter = TraverseBvhIteratorCpu {
              bvh: &self.tri_bvh_forest,
              ray_origin: blas_ray_origin,
              ray_direction: blas_ray_direction,
              ray_range: blas_ray_range.clone(),
              curr_idx: blas_root_idx,
            };

            for hit_idx in bvh_iter {
              let node = &self.tri_bvh_forest[hit_idx as usize];

              for tri_idx in node.content_range.x..node.content_range.y {
                let i0 = self.indices[tri_idx as usize * 3];
                let i1 = self.indices[tri_idx as usize * 3 + 1];
                let i2 = self.indices[tri_idx as usize * 3 + 2];
                let v0 = self.vertices[i0 as usize];
                let v1 = self.vertices[i1 as usize];
                let v2 = self.vertices[i2 as usize];

                TRI_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // vec4(hit, distance, u, v)
                let intersection = intersect_ray_triangle_cpu(
                  blas_ray_origin,
                  blas_ray_direction,
                  blas_ray_range.get(),
                  v0,
                  v1,
                  v2,
                  cull_enable,
                  cull_back,
                );

                if intersection[0] > 0. {
                  let distance = intersection[1] / distance_scaling;
                  let p = blas_ray_origin + distance * blas_ray_direction;
                  // println!("hit {p:?}");
                  let primitive_idx = tri_idx - primitive_start;
                  // opaque -> anyhit, non-opaque -> intersect
                  // assuming all opaque
                  TRI_HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                  let accepted = any_hit(geometry_idx, primitive_idx, distance, p);
                  if accepted {
                    ray_range.update_far(distance);
                  }
                }
              }
            }
          }
        }

        // if flags.visit_boxes_cpu() {
        //   let blas_ray_range = ray_range.clone_with_scaling(distance_scaling);
        //   for box_root_index in blas_meta_info.box_root_range.x..blas_meta_info.box_root_range.y {
        //     let geometry = self.box_bvh_root[box_root_index as usize];
        //     let blas_root_idx = geometry.bvh_root_idx;
        //     let _geometry_idx = geometry.geometry_idx;
        //     let _primitive_start = geometry.primitive_start;
        //     let geometry_flags = geometry.geometry_flags;
        //
        //     let (pass, _is_opaque) = TraverseFlags::cull_geometry_cpu(flags, geometry_flags);
        //     if !pass {
        //       continue;
        //     }
        //
        //     let box_iter = TraverseBvhIteratorCpu {
        //       bvh: &self.box_bvh_forest,
        //       ray_origin: blas_ray_origin,
        //       ray_direction: blas_ray_direction,
        //       ray_range: blas_ray_range.clone(),
        //       curr_idx: blas_root_idx,
        //     };
        //
        //     for hit_idx in box_iter {
        //       let node = &self.box_bvh_forest[hit_idx as usize];
        //       let aabb =
        //         &self.boxes[node.content_range.x as usize * 2..node.content_range.y as usize * 2];
        //       for aabb in aabb.chunks_exact(2) {
        //         let hit = intersect_ray_aabb_cpu(
        //           blas_ray_origin,
        //           blas_ray_direction,
        //           blas_ray_range.get(),
        //           aabb[0],
        //           aabb[1],
        //         );
        //         if hit {
        //           // todo call intersect, then anyhit
        //           // todo modify range after hit
        //         }
        //       }
        //     }
        //   }
        // }
      }
    }
  }
}

use std::cell::Cell;
use std::rc::Rc;
#[derive(Clone)]
pub(crate) struct RayRange {
  near: f32,
  far: Rc<Cell<f32>>,
  scaling: f32,
}
impl RayRange {
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
  ray_range: RayRange,

  curr_idx: u32,
}
impl<'a> Iterator for TraverseBvhIteratorCpu<'a> {
  type Item = u32;
  fn next(&mut self) -> Option<Self::Item> {
    while self.curr_idx != INVALID_NEXT {
      BVH_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
          // is leaf
          BVH_HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
          return Some(curr);
        }
      } else {
        self.curr_idx = node.miss_next;
      };
    }

    None
  }
}
