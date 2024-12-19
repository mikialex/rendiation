use bytemuck::Zeroable;
use rendiation_algebra::{vec2, Vec2, Vec3};
use rendiation_geometry::Box3;
use rendiation_space_algorithm::bvh::{FlattenBVH, FlattenBVHNode, SAH};
use rendiation_space_algorithm::utils::TreeBuildOption;

use crate::backend::intersect_ray_aabb_cpu;
use crate::backend::wavefront_compute::geometry::intersect_ray_triangle_cpu;
use crate::backend::wavefront_compute::geometry::naive::flag::TraverseFlags;
use crate::backend::wavefront_compute::geometry::naive::traverse_cpu::{
  RayRange, BVH_HIT_COUNT, BVH_VISIT_COUNT, TRI_HIT_COUNT, TRI_VISIT_COUNT,
};
use crate::backend::wavefront_compute::geometry::naive::{
  compute_bvh_next, DeviceBVHNode, INVALID_NEXT,
};
use crate::{
  BottomLevelAccelerationStructureBuildBuffer, BottomLevelAccelerationStructureBuildSource,
  GeometryFlags, RayAnyHitBehavior, ANYHIT_BEHAVIOR_ACCEPT_HIT, ANYHIT_BEHAVIOR_END_SEARCH,
};

fn flatten_bvh_to_gpu_node(node: FlattenBVHNode<Box3>, hit: u32, miss: u32) -> DeviceBVHNode {
  DeviceBVHNode {
    aabb_min: node.bounding.min,
    aabb_max: node.bounding.max,
    hit_next: hit,
    miss_next: miss,
    content_range: vec2(
      node.primitive_range.start as u32,
      node.primitive_range.end as u32,
    ),
    ..Zeroable::zeroed()
  }
}

#[derive(Copy, Clone)]
struct BlasMeta {
  geometry_count: u32,
  geometry_offset: u32,
  bvh_offset: u32,
  primitives_offset: u32,
  vertices_offset: u32,
}
#[derive(Copy, Clone)]
struct GeometryMeta {
  bvh_root: u32,
  geometry_flags: GeometryFlags,
  geometry_idx: u32,
  primitives_offset: u32,
  vertices_offset: u32,
}

/// save offsets in meta, pack buffers with no modification
pub struct BuiltBlasPack {
  blas_meta: Vec<BlasMeta>, // length = blas count

  geometry_meta: Vec<GeometryMeta>, // length = geometry count

  bvh: Vec<DeviceBVHNode>, // next = hit/miss + root of geometry_idx

  indices_redirect: Vec<u32>, // bvh node index -> primitive id
  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
}
impl BuiltBlasPack {
  pub fn build(sources: &[Option<Vec<BottomLevelAccelerationStructureBuildSource>>]) -> Self {
    let built_blas_list = sources
      .iter()
      .map(|source| {
        if let Some(source) = source {
          let built_geometry_list = source
            .iter()
            .enumerate()
            .filter_map(|(geometry_idx, geometry)| match &geometry.geometry {
              BottomLevelAccelerationStructureBuildBuffer::Triangles { positions, indices } => {
                Some(BuiltGeometry::build(
                  geometry_idx as u32,
                  geometry.flags,
                  positions,
                  indices,
                ))
              }
              BottomLevelAccelerationStructureBuildBuffer::AABBs { .. } => None,
            })
            .collect::<Vec<_>>();
          BuiltGeometryPack::pack(built_geometry_list)
        } else {
          let geometry = BuiltGeometry::default();
          BuiltGeometryPack::pack(vec![geometry])
        }
      })
      .collect();

    Self::pack(built_blas_list)
  }

  fn pack(blas: Vec<BuiltGeometryPack>) -> Self {
    let mut blas_meta = vec![];
    let mut bvh = vec![];
    let mut geometry_meta = vec![];
    let mut indices_redirect = vec![];
    let mut indices = vec![];
    let mut vertices = vec![];
    for built_blas in blas {
      let indices_offset = indices.len() as u32;
      assert_eq!(0, indices_offset % 3);
      blas_meta.push(BlasMeta {
        geometry_offset: geometry_meta.len() as u32,
        geometry_count: built_blas.geometry_meta.len() as u32,
        bvh_offset: bvh.len() as u32,
        primitives_offset: indices_offset / 3,
        vertices_offset: vertices.len() as u32,
      });
      bvh.extend(built_blas.bvh);
      geometry_meta.extend(built_blas.geometry_meta);
      indices_redirect.extend(built_blas.indices_redirect);
      indices.extend(built_blas.indices);
      vertices.extend(built_blas.vertices);
    }
    Self {
      blas_meta,
      bvh,
      geometry_meta,
      indices_redirect,
      indices,
      vertices,
    }
  }
}

/// save offsets in meta, pack buffers with no modification
struct BuiltGeometryPack {
  geometry_meta: Vec<GeometryMeta>, // length = geometry count

  bvh: Vec<DeviceBVHNode>,

  indices_redirect: Vec<u32>,
  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
}
impl BuiltGeometryPack {
  fn pack(built_geometry_triangles: Vec<BuiltGeometry>) -> Self {
    let mut bvh = vec![];
    let mut geometry_meta = vec![];
    let mut indices_redirect = vec![];
    let mut indices = vec![];
    let mut vertices = vec![];
    // todo optimize for single geometry
    for built_geometry in built_geometry_triangles {
      let indices_offset = indices.len() as u32;
      assert_eq!(0, indices_offset % 3);
      geometry_meta.push(GeometryMeta {
        bvh_root: bvh.len() as u32,
        geometry_flags: built_geometry.geometry_flags,
        geometry_idx: built_geometry.geometry_idx,
        primitives_offset: indices_offset / 3,
        vertices_offset: vertices.len() as u32,
      });
      bvh.extend(built_geometry.bvh);
      indices_redirect.extend(built_geometry.indices_redirect);
      indices.extend(built_geometry.indices);
      vertices.extend(built_geometry.vertices);
    }
    Self {
      bvh,
      geometry_meta,
      indices_redirect,
      indices,
      vertices,
    }
  }
}

#[derive(Default)]
struct BuiltGeometry {
  geometry_flags: GeometryFlags,
  geometry_idx: u32,
  bvh: Vec<DeviceBVHNode>,
  indices_redirect: Vec<u32>,
  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
}
impl BuiltGeometry {
  fn build(
    geometry_idx: u32,
    flags: GeometryFlags,
    vertices: &Vec<Vec3<f32>>,
    indices: &Option<Vec<u32>>,
  ) -> Self {
    // if non-indexed, create a Vec<u32> as indices. this will be memory consuming.
    let indices = match indices.as_ref() {
      Some(indices) => indices.clone(),
      None => (0..vertices.len() as u32).collect::<Vec<u32>>(),
    };

    let boxes = indices.chunks_exact(3).map(|triangle| {
      triangle
        .iter()
        .map(|idx| vertices[*idx as usize])
        .collect::<Box3>()
    });

    let bvh = FlattenBVH::new(
      boxes,
      &mut SAH::new(4),
      &TreeBuildOption {
        max_tree_depth: 50,
        bin_size: 2,
      },
    );
    let bvh_nodes = bvh.nodes;
    let hit_miss = compute_bvh_next(&bvh_nodes);
    let geometry_indices_redirect = bvh
      .sorted_primitive_index
      .into_iter()
      .map(|i| i as u32)
      .collect();

    let bvh_nodes = bvh_nodes
      .into_iter()
      .zip(hit_miss)
      .map(|(node, (hit, miss))| flatten_bvh_to_gpu_node(node, hit, miss))
      .collect::<Vec<_>>();

    Self {
      geometry_flags: flags,
      geometry_idx,
      bvh: bvh_nodes,
      indices_redirect: geometry_indices_redirect,
      indices,
      vertices: vertices.clone(),
    }
  }
}

/// general bvh traversal, return hit idx (before redirection)
struct TraverseBvhIteratorCpu2<'a> {
  bvh: &'a [DeviceBVHNode],
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: RayRange,
  bvh_offset: u32,

  curr_idx: u32,
}
impl<'a> Iterator for TraverseBvhIteratorCpu2<'a> {
  type Item = u32;
  fn next(&mut self) -> Option<u32> {
    while self.curr_idx != INVALID_NEXT {
      BVH_VISIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      let node = &self.bvh[(self.curr_idx + self.bvh_offset) as usize];
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
          return Some(curr + self.bvh_offset);
        }
      } else {
        self.curr_idx = node.miss_next;
      };
    }

    None
  }
}

pub struct HitPoint {
  pub geometry_idx: u32,
  pub primitive_idx: u32,
  pub distance: f32,
  pub position: Vec3<f32>,
  pub uv: Vec2<f32>,
  pub is_opaque: bool,
}

impl BuiltBlasPack {
  /// returns end_search
  pub fn intersect_blas_cpu(
    &self,
    blas_idx: u32,
    ray_origin: Vec3<f32>,
    ray_direction: Vec3<f32>,
    ray_range: RayRange,
    distance_scaling: f32,
    flags: TraverseFlags,
    on_hit: &mut impl FnMut(HitPoint) -> RayAnyHitBehavior,
  ) -> bool {
    let blas = &self.blas_meta[blas_idx as usize];

    for geometry_idx in 0..blas.geometry_count {
      let index = (blas.geometry_offset + geometry_idx) as usize;
      let meta = self.geometry_meta[index];

      assert_eq!(meta.geometry_idx, geometry_idx);
      let geometry_meta = GeometryMeta {
        bvh_root: meta.bvh_root + blas.bvh_offset,
        geometry_flags: meta.geometry_flags,
        geometry_idx: meta.geometry_idx,
        primitives_offset: meta.primitives_offset + blas.primitives_offset,
        vertices_offset: meta.vertices_offset + blas.vertices_offset,
      };

      let (pass, is_opaque) = TraverseFlags::cull_geometry(flags, geometry_meta.geometry_flags);
      if !pass {
        continue;
      }
      let (cull_enable, cull_back) = TraverseFlags::cull_triangle(flags);

      let iter_bvh = TraverseBvhIteratorCpu2 {
        bvh: &self.bvh,
        ray_origin,
        ray_direction,
        ray_range: ray_range.clone(),
        bvh_offset: geometry_meta.bvh_root, // root is offset
        curr_idx: 0,                        // start from first local node
      };
      for bvh_node_idx in iter_bvh {
        let node = self.bvh[bvh_node_idx as usize];
        let start = node.content_range.x + geometry_meta.primitives_offset;
        let end = node.content_range.y + geometry_meta.primitives_offset;

        for primitive_idx in start..end {
          let primitive_idx_raw = self.indices_redirect[primitive_idx as usize];
          let primitive_idx = (primitive_idx_raw + geometry_meta.primitives_offset) as usize;
          let a = self.indices[primitive_idx * 3] + geometry_meta.vertices_offset;
          let b = self.indices[primitive_idx * 3 + 1] + geometry_meta.vertices_offset;
          let c = self.indices[primitive_idx * 3 + 2] + geometry_meta.vertices_offset;
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
              primitive_idx: primitive_idx_raw,
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