use bytemuck::Zeroable;
use rendiation_algebra::{vec2, Vec2, Vec3};
use rendiation_geometry::Box3;
use rendiation_shader_derives::{std430_layout, ShaderStruct};
use rendiation_space_algorithm::bvh::{FlattenBVH, FlattenBVHNode, SAH};
use rendiation_space_algorithm::utils::TreeBuildOption;

use crate::backend::wavefront_compute::geometry::naive::traverse_gpu::BuiltBlasPackGpu;
use crate::backend::wavefront_compute::geometry::naive::{compute_bvh_next, DeviceBVHNode};
use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct GeometryMeta {
  pub bvh_root: u32,
  pub geometry_flags: GeometryFlags,
  pub geometry_idx: u32,
  pub primitives_offset: u32,
  pub vertices_offset: u32,
}
#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct BlasMeta {
  pub geometry_count: u32,
  pub geometry_offset: u32,
  pub bvh_offset: u32,
  pub primitives_offset: u32,
  pub vertices_offset: u32,
}

#[derive(Default)]
struct BuiltGeometry {
  geometry_flags: GeometryFlags,
  geometry_idx: u32,
  bvh: Vec<DeviceBVHNode>,
  primitive_redirect: Vec<u32>,
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
    let geometry_primitive_redirect = bvh
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
      primitive_redirect: geometry_primitive_redirect,
      indices,
      vertices: vertices.clone(),
    }
  }
}

/// save offsets in meta, pack buffers with no modification
struct BuiltGeometryPack {
  bounding: Box3,

  geometry_meta: Vec<GeometryMeta>, // length = geometry count

  bvh: Vec<DeviceBVHNode>,

  primitive_redirect: Vec<u32>,
  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
}
impl BuiltGeometryPack {
  pub fn build(source: &[BottomLevelAccelerationStructureBuildSource]) -> Self {
    let built_geometry_list = source
      .iter()
      .enumerate()
      .filter_map(|(geometry_idx, geometry)| match &geometry.geometry {
        BottomLevelAccelerationStructureBuildBuffer::Triangles { positions, indices } => Some(
          BuiltGeometry::build(geometry_idx as u32, geometry.flags, positions, indices),
        ),
        BottomLevelAccelerationStructureBuildBuffer::AABBs { .. } => None,
      })
      .collect::<Vec<_>>();
    Self::pack(built_geometry_list)
  }

  fn pack(built_geometry_triangles: Vec<BuiltGeometry>) -> Self {
    let mut bounding = Box3::default();
    let mut bvh = vec![];
    let mut geometry_meta = vec![];
    let mut primitive_redirect = vec![];
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
        ..Zeroable::zeroed()
      });
      bounding.expand_by_other(Box3::new(
        built_geometry.bvh[0].aabb_min,
        built_geometry.bvh[0].aabb_max,
      ));
      bvh.extend(built_geometry.bvh);
      primitive_redirect.extend(built_geometry.primitive_redirect);
      indices.extend(built_geometry.indices);
      vertices.extend(built_geometry.vertices);
    }
    Self {
      bounding,
      bvh,
      geometry_meta,
      primitive_redirect,
      indices,
      vertices,
    }
  }
}

#[derive(Default)]
pub struct BuiltBlasPool {
  blas: Vec<BuiltGeometryPack>,

  result: Option<(Arc<BuiltBlasPack>, Arc<BuiltBlasPackGpu>)>,
}

impl BuiltBlasPool {
  pub fn create(
    &mut self,
    idx: usize,
    blas_source: &[BottomLevelAccelerationStructureBuildSource],
  ) {
    let pack = BuiltGeometryPack::build(blas_source);
    if self.blas.len() <= idx {
      self.blas.push(pack);
    } else {
      self.blas[idx] = pack;
    }
    self.result = None;
  }
  pub fn delete(&mut self, idx: usize) {
    self.blas[idx] = BuiltGeometryPack::build(&[]);
    // no invalidation
  }

  pub fn _shrink(&mut self) {
    todo!()
  }

  pub fn get(&mut self, device: &GPUDevice) -> &(Arc<BuiltBlasPack>, Arc<BuiltBlasPackGpu>) {
    if self.result.is_none() {
      let cpu = BuiltBlasPack::pack(&self.blas);
      let gpu = cpu.build_gpu(device);
      self.result = Some((Arc::new(cpu), Arc::new(gpu)));
    }
    self.result.as_ref().unwrap()
  }
}

/// save offsets in meta, pack buffers with no modification
pub struct BuiltBlasPack {
  pub blas_bounding: Vec<Box3>, // length = blas count, read by tlas
  pub blas_meta: Vec<BlasMeta>, // length = blas count

  pub geometry_meta: Vec<GeometryMeta>, // length = geometry count

  pub bvh: Vec<DeviceBVHNode>, // next = hit/miss + root of geometry_idx

  pub primitive_redirect: Vec<u32>, // bvh node index -> primitive id
  pub indices: Vec<u32>,
  pub vertices: Vec<Vec3<f32>>,
}
impl BuiltBlasPack {
  fn pack(blas: &[BuiltGeometryPack]) -> Self {
    let mut blas_bounding = vec![];
    let mut blas_meta = vec![];
    let mut bvh = vec![];
    let mut geometry_meta = vec![];
    let mut primitive_redirect = vec![];
    let mut indices = vec![];
    let mut vertices = vec![];
    for built_blas in blas {
      blas_bounding.push(built_blas.bounding);
      let indices_offset = indices.len() as u32;
      assert_eq!(0, indices_offset % 3);
      blas_meta.push(BlasMeta {
        geometry_offset: geometry_meta.len() as u32,
        geometry_count: built_blas.geometry_meta.len() as u32,
        bvh_offset: bvh.len() as u32,
        primitives_offset: indices_offset / 3,
        vertices_offset: vertices.len() as u32,
        ..Zeroable::zeroed()
      });
      bvh.extend_from_slice(&built_blas.bvh);
      geometry_meta.extend_from_slice(&built_blas.geometry_meta);
      primitive_redirect.extend_from_slice(&built_blas.primitive_redirect);
      indices.extend_from_slice(&built_blas.indices);
      vertices.extend_from_slice(&built_blas.vertices);
    }
    Self {
      blas_bounding,
      blas_meta,
      bvh,
      geometry_meta,
      primitive_redirect,
      indices,
      vertices,
    }
  }
}

#[allow(unused)]
pub struct HitPoint {
  pub geometry_idx: u32,
  pub primitive_idx: u32,
  pub distance: f32,
  pub position: Vec3<f32>,
  pub uv: Vec2<f32>,
  pub is_opaque: bool,
}
