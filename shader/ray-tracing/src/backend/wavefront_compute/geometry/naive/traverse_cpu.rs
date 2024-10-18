use rendiation_algebra::Vec3;

use crate::backend::wavefront_compute::geometry::intersect_ray_triangle_cpu;
use crate::backend::wavefront_compute::geometry::naive::*;

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

impl NaiveSahBvhCpu {
  pub(super) fn traverse(
    &self,
    ray: &mut ShaderRayTraceCallStoragePayload,
    any_hit: &mut dyn FnMut(u32, u32, f32, Vec3<f32>), /* geometry_idx, primitive_idx, distance, hit_position // todo use ctx */
  ) {
    let flags = TraverseFlags::from_ray_flag_cpu(ray.ray_flags);

    // traverse tlas bvh, hit leaf
    let tlas_iter = TraverseBvhIteratorCpu {
      bvh: &self.tlas_bvh_forest,
      ray_origin: ray.ray_origin,
      ray_direction: ray.ray_direction,
      ray_range: ray.range,
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
        let flags = flags.apply_geometry_instance_flag_cpu(tlas_data.flags);

        // traverse blas bvh
        let blas_ray_origin = tlas_data.transform_inv * ray.ray_origin.expand_with_one();
        let blas_ray_origin = blas_ray_origin.xyz() / blas_ray_origin.w();
        let blas_ray_direction = tlas_data.transform_inv.to_mat3() * ray.ray_direction;
        let distance_scaling = blas_ray_direction.length();
        let blas_ray_range = ray.range * distance_scaling;
        let blas_ray_direction = blas_ray_direction.normalize();

        let blas_meta_info = &self.blas_meta_info[blas_idx as usize];

        let skip_triangles = (flags as u32 & TraverseFlags::SKIP_TRIANGLES as u32) > 0;
        if !skip_triangles {
          for tri_root_index in blas_meta_info.tri_root_range.x..blas_meta_info.tri_root_range.y {
            let geometry = self.tri_bvh_root[tri_root_index as usize];
            let blas_root_idx = geometry.bvh_root_idx;
            let geometry_idx = geometry.geometry_idx;
            let primitive_start = geometry.primitive_start;
            let geometry_flags = geometry.geometry_flags;
            // todo apply flags, cull

            let bvh_iter = TraverseBvhIteratorCpu {
              bvh: &self.tri_bvh_forest,
              ray_origin: blas_ray_origin,
              ray_direction: blas_ray_direction,
              ray_range: blas_ray_range,
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

                // vec4(hit, distance, u, v)
                let intersection = intersect_ray_triangle_cpu(
                  blas_ray_origin,
                  blas_ray_direction,
                  blas_ray_range,
                  v0,
                  v1,
                  v2,
                  // todo check flags
                );

                if intersection[0] > 0. {
                  let distance = intersection[1] / distance_scaling;
                  let p = blas_ray_origin + distance * blas_ray_direction;
                  // println!("hit {p:?}");
                  let primitive_idx = tri_idx - primitive_start;
                  any_hit(geometry_idx, primitive_idx, distance, p);
                }
              }
            }
          }
        }

        let skip_boxes = (flags as u32 & TraverseFlags::SKIP_BOXES as u32) > 0;
        if !skip_boxes {
          for box_root_index in blas_meta_info.box_root_range.x..blas_meta_info.box_root_range.y {
            let idx = self.box_bvh_root[box_root_index as usize];
            let blas_root_idx = idx.bvh_root_idx;
            // let geometry_idx = idx.geometry_idx;

            let box_iter = TraverseBvhIteratorCpu {
              bvh: &self.box_bvh_forest,
              ray_origin: blas_ray_origin,
              ray_direction: blas_ray_direction,
              ray_range: blas_ray_range,
              curr_idx: blas_root_idx,
            };

            for hit_idx in box_iter {
              let node = &self.box_bvh_forest[hit_idx as usize];
              let aabb =
                &self.boxes[node.content_range.x as usize * 2..node.content_range.y as usize * 2];
              for aabb in aabb.chunks_exact(2) {
                let hit = intersect_ray_aabb_cpu(
                  blas_ray_origin,
                  blas_ray_direction,
                  blas_ray_range,
                  aabb[0],
                  aabb[1],
                );
                if hit {
                  // todo call intersect, then anyhit
                  // todo modify range after hit
                }
              }
            }
          }
        }
      }
    }
  }
}

struct TraverseBvhIteratorCpu<'a> {
  bvh: &'a [DeviceBVHNode],
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: Vec2<f32>,

  curr_idx: u32,
}
impl<'a> Iterator for TraverseBvhIteratorCpu<'a> {
  type Item = u32;
  fn next(&mut self) -> Option<Self::Item> {
    while self.curr_idx != INVALID_NEXT {
      let node = &self.bvh[self.curr_idx as usize];
      if intersect_ray_aabb_cpu(
        self.ray_origin,
        self.ray_direction,
        self.ray_range,
        node.aabb_min,
        node.aabb_max,
      ) {
        let curr = self.curr_idx;
        self.curr_idx = node.hit_next;

        if node.hit_next == node.miss_next {
          // is leaf
          return Some(curr);
        }
      } else {
        self.curr_idx = node.miss_next;
      };
    }

    None
  }
}
