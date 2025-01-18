#[cfg(test)]
mod test;
#[cfg(test)]
pub(crate) use test::*;

mod flag;
mod traverse_cpu;
mod traverse_gpu;

use std::ops::{BitAnd, Deref};
use std::sync::{RwLock, RwLockReadGuard};

use flag::*;
use rendiation_geometry::Box3;
use rendiation_space_algorithm::bvh::*;
use rendiation_space_algorithm::utils::TreeBuildOption;
use traverse_cpu::*;
use traverse_gpu::*;

use crate::backend::wavefront_compute::geometry::{intersect_ray_triangle_gpu, Ray};
use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct, StorageNodePtrAccess)]
pub struct TopLevelAccelerationStructureSourceDeviceInstance {
  pub transform: Mat4<f32>,
  pub transform_inv: Mat4<f32>,
  pub instance_custom_index: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: u32,
  pub acceleration_structure_handle: u32, // blas index
}
#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
pub struct TlasBounding {
  pub world_min: Vec3<f32>,
  pub mask: u32,
  pub world_max: Vec3<f32>,
  pub flags: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
struct DeviceBVHNode {
  pub aabb_min: Vec3<f32>,
  pub hit_next: u32,
  pub aabb_max: Vec3<f32>,
  pub miss_next: u32,
  pub content_range: Vec2<u32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
struct BlasMetaInfo {
  pub tri_root_range: Vec2<u32>,
  // pub box_root_range: Vec2<u32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
struct GeometryMetaInfo {
  pub bvh_root_idx: u32,
  pub geometry_idx: u32,
  pub primitive_start: u32,
  pub geometry_flags: u32,
}

#[derive(Default)]
struct NaiveSahBvhSource {
  blas_data: Vec<Option<Vec<BottomLevelAccelerationStructureBuildSource>>>,
  tlas_data: Vec<Option<Vec<TopLevelAccelerationStructureSourceInstance>>>,
}

struct BuiltBlas {
  blas_meta_info: Vec<BlasMetaInfo>,
  blas_box: Vec<Box3>,

  // (tri_bvh, hit miss, primitive_offset, geometry_idx, geometry_flags)
  tri_bvh: Vec<(FlattenBVH<Box3>, Vec<(u32, u32)>, u32, u32, u32)>,
  // (box_bvh, hit miss, box_offset, geometry_idx, geometry_flags)
  // box_bvh: Vec<(FlattenBVH<Box3>, Vec<(u32, u32)>, u32, u32, u32)>,

  // map bvh sorted triangle id to source triangle id
  indices_redirect: Vec<u32>,
  // directly index vertices, no offset required
  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
  // boxes_redirect: Vec<f32>,
  // boxes: Vec<Vec3<f32>>,
}

impl NaiveSahBvhSource {
  pub fn create_blas(&mut self, source: &[BottomLevelAccelerationStructureBuildSource]) -> u32 {
    // todo freelist
    let index = self.blas_data.len();
    self.blas_data.push(Some(source.to_vec()));
    index as u32
  }
  pub fn create_tlas(&mut self, source: &[TopLevelAccelerationStructureSourceInstance]) -> u32 {
    // todo freelist
    let start_index = self.tlas_data.len();
    self.tlas_data.push(Some(source.to_vec()));
    start_index as u32
  }
  pub fn delete_blas(&mut self, handle: BlasHandle) {
    // todo freelist
    let idx = handle.0;
    self.blas_data[idx as usize] = None;
  }
  pub fn delete_tlas(&mut self, handle: TlasHandle) {
    // todo freelist
    let idx = handle.0;
    self.tlas_data[idx as usize] = None;
  }

  // todo incremental change
  fn build_blas(
    blas_data: &[Option<Vec<BottomLevelAccelerationStructureBuildSource>>],
  ) -> BuiltBlas {
    let mut geometry_indices_redirect = vec![];
    let mut geometry_indices = vec![];
    let mut geometry_vertices = vec![];
    // let mut geometry_boxes_redirect = vec![];
    // let mut geometry_boxes = vec![];

    let mut blas_meta_info = vec![];
    let mut blas_box = vec![];
    let mut tri_bvh = vec![];
    // let mut box_bvh = vec![];

    for blas in blas_data {
      if blas.is_none() {
        blas_meta_info.push(BlasMetaInfo::zeroed());
        continue;
      }
      let blas = blas.as_ref().unwrap();

      let tri_start = tri_bvh.len();
      // let box_start = box_bvh.len();

      // todo par_iter
      for (geometry_idx, source) in blas.iter().enumerate() {
        let geometry_idx = geometry_idx as u32;
        let mut root_box = Box3::default();
        let geometry_flags = source.flags;
        match &source.geometry {
          BottomLevelAccelerationStructureBuildBuffer::Triangles { positions, indices } => {
            let primitive_start = geometry_indices.len() as u32 / 3;
            let vertex_start = geometry_vertices.len() as u32;

            use std::borrow::Cow;
            // if non-indexed, create a Vec<u32> as indices. this will be memory consuming.
            let indices = match indices.as_ref() {
              None => Cow::Owned((0..positions.len() as u32).collect::<Vec<u32>>()),
              Some(indices) => Cow::Borrowed(indices.as_slice()),
            };

            geometry_vertices.extend_from_slice(positions);

            let boxes = indices.chunks_exact(3).map(|triangle| {
              triangle
                .iter()
                .map(|idx| positions[*idx as usize])
                .collect::<Box3>()
            });

            let option = TreeBuildOption {
              max_tree_depth: 50,
              bin_size: 2,
            };
            let mut sah = SAH::new(4);
            let bvh = FlattenBVH::new(boxes, &mut sah, &option);
            root_box.expand_by_other(bvh.nodes[0].bounding);
            let traverse_next = compute_bvh_next(&bvh.nodes);

            let raw_primitive_start = geometry_indices.len() as u32 / 3;
            for &primitive_idx in &bvh.sorted_primitive_index {
              geometry_indices_redirect.push(raw_primitive_start + primitive_idx as u32);
            }
            indices.chunks_exact(3).for_each(|triangle| {
              geometry_indices.push(vertex_start + triangle[0]);
              geometry_indices.push(vertex_start + triangle[1]);
              geometry_indices.push(vertex_start + triangle[2]);
            });

            tri_bvh.push((
              bvh,
              traverse_next,
              primitive_start,
              geometry_idx,
              geometry_flags,
            ));
          }

          BottomLevelAccelerationStructureBuildBuffer::AABBs { aabbs: _ } => {
            // let boxes_start = geometry_indices.len() as u32;
            //
            // let boxes = aabbs.iter().map(|aabb| {
            //   let mut r = Box3::default();
            //   let point0 = vec3(aabb[0], aabb[1], aabb[2]);
            //   let point1 = vec3(aabb[3], aabb[4], aabb[5]);
            //   r.expand_by_point(point0);
            //   r.expand_by_point(point1);
            //   r
            // });
            //
            // let option = TreeBuildOption {
            //   max_tree_depth: 50,
            //   bin_size: 2,
            // };
            // let mut sah = SAH::new(4);
            // let bvh = FlattenBVH::new(boxes, &mut sah, &option);
            // root_box.expand_by_other(bvh.nodes[0].bounding);
            // let traverse_next = compute_bvh_next(&bvh.nodes);
            //
            // for primitive_idx in &bvh.sorted_primitive_index {
            //   let aabb = &aabbs[*primitive_idx];
            //   let point0 = vec3(aabb[0], aabb[1], aabb[2]);
            //   let point1 = vec3(aabb[3], aabb[4], aabb[5]);
            //   geometry_boxes.push(point0);
            //   geometry_boxes.push(point1);
            // }
            //
            // box_bvh.push((
            //   bvh,
            //   traverse_next,
            //   boxes_start,
            //   geometry_idx,
            //   geometry_flags,
            // ));
          }
        }
        blas_box.push(root_box);
      }

      let tri_end = tri_bvh.len();
      // let box_end = box_bvh.len();

      blas_meta_info.push(BlasMetaInfo {
        tri_root_range: vec2(tri_start as u32, tri_end as u32),
        // box_root_range: vec2(box_start as u32, box_end as u32),
        ..Zeroable::zeroed()
      });
    }

    BuiltBlas {
      blas_meta_info,
      blas_box,
      tri_bvh,
      indices_redirect: geometry_indices_redirect,
      indices: geometry_indices,
      vertices: geometry_vertices,
    }
  }

  fn build_tlas(
    tlas_data: &[TopLevelAccelerationStructureSourceInstance],
    blas_box: &[Box3],
    tlas_items_out: &mut Vec<TopLevelAccelerationStructureSourceDeviceInstance>,
    tlas_boundings_out: &mut Vec<TlasBounding>,
  ) -> (FlattenBVH<Box3>, Vec<(u32, u32)>) {
    let mut tlas_bvh_aabb = vec![];
    let mut index_mapping = vec![]; // tlas_data[index_mapping[idx]] aabb = bvh.nodes[idx].bounding

    for (idx, source) in tlas_data.iter().enumerate() {
      let blas_idx = source.acceleration_structure_handle.0 as usize;
      let aabb = blas_box[blas_idx].apply_matrix_into(source.transform);
      index_mapping.push(idx);
      tlas_bvh_aabb.push(aabb);
    }

    let option = TreeBuildOption {
      max_tree_depth: 50,
      bin_size: 10,
    };
    let mut sah = SAH::new(4);
    let bvh = FlattenBVH::new(tlas_bvh_aabb.clone().into_iter(), &mut sah, &option);
    let traverse_next = compute_bvh_next(&bvh.nodes);

    for box_idx in &bvh.sorted_primitive_index {
      let aabb = tlas_bvh_aabb[*box_idx];
      let tlas_idx = index_mapping[*box_idx];
      let source = &tlas_data[tlas_idx];

      let mut flags = source.flags;
      if source.transform.to_mat3().det() < 0. {
        flags ^= GEOMETRY_INSTANCE_TRIANGLE_FLIP_FACING;
      }

      let tlas_item = TopLevelAccelerationStructureSourceDeviceInstance {
        transform: source.transform,
        transform_inv: source.transform.inverse_or_identity(),
        instance_custom_index: source.instance_custom_index,
        instance_shader_binding_table_record_offset: source
          .instance_shader_binding_table_record_offset,
        flags,
        acceleration_structure_handle: source.acceleration_structure_handle.0,
        ..Zeroable::zeroed()
      };
      let tlas_bounding = TlasBounding {
        world_min: aabb.min,
        world_max: aabb.max,
        mask: source.mask,
        flags,
        ..Zeroable::zeroed()
      };
      tlas_items_out.push(tlas_item);
      tlas_boundings_out.push(tlas_bounding);
    }

    (bvh, traverse_next)
  }

  pub fn build(
    &mut self,
    device: &GPUDevice,
    cpu_data: &mut Option<NaiveSahBvhCpu>,
    gpu_data: &mut Option<NaiveSahBvhGpu>,
  ) {
    // build blas
    let BuiltBlas {
      blas_meta_info,
      blas_box,
      tri_bvh,
      indices_redirect,
      indices,
      vertices,
    } = Self::build_blas(&self.blas_data);
    fn flatten_bvh_to_gpu_node(
      node: &FlattenBVHNode<Box3>,
      hit: u32,
      miss: u32,
      next_offset: u32,
      primitive_offset: u32,
    ) -> DeviceBVHNode {
      let hit = if hit != INVALID_NEXT {
        hit + next_offset
      } else {
        INVALID_NEXT
      };
      let miss = if miss != INVALID_NEXT {
        miss + next_offset
      } else {
        INVALID_NEXT
      };

      DeviceBVHNode {
        aabb_min: node.bounding.min,
        aabb_max: node.bounding.max,
        hit_next: hit,
        miss_next: miss,
        content_range: vec2(
          node.primitive_range.start as u32 + primitive_offset,
          node.primitive_range.end as u32 + primitive_offset,
        ),
        ..Zeroable::zeroed()
      }
    }
    let mut tri_bvh_root = vec![];
    // let mut box_bvh_root = vec![];
    let mut tri_bvh_forest = vec![];
    // let mut box_bvh_forest = vec![];
    for (bvh, hit_miss, primitive_start, geometry_idx, geometry_flags) in tri_bvh {
      let bvh_start = tri_bvh_forest.len() as u32;
      tri_bvh_root.push(GeometryMetaInfo {
        bvh_root_idx: bvh_start,
        geometry_idx,
        primitive_start,
        geometry_flags,
        ..Zeroable::zeroed()
      });
      let nodes = bvh.nodes.iter().zip(hit_miss).map(|(node, (hit, miss))| {
        flatten_bvh_to_gpu_node(node, hit, miss, bvh_start, primitive_start)
      });
      tri_bvh_forest.extend(nodes);
    }
    // for (bvh, hit_miss, primitive_start, geometry_idx, geometry_flags) in box_bvh {
    //   let bvh_start = box_bvh_forest.len() as u32;
    //   box_bvh_root.push(GeometryMetaInfo {
    //     bvh_root_idx: bvh_start,
    //     geometry_idx,
    //     primitive_start,
    //     geometry_flags,
    //     ..Zeroable::zeroed()
    //   });
    //   let nodes = bvh.nodes.iter().zip(hit_miss).map(|(node, (hit, miss))| {
    //     flatten_bvh_to_gpu_node(node, hit, miss, bvh_start, primitive_start)
    //   });
    //   box_bvh_forest.extend(nodes);
    // }

    fn create_gpu_buffer<T>(device: &GPUDevice, data: &[T]) -> StorageBufferReadOnlyDataView<[T]>
    where
      [T]: Std430MaybeUnsized,
      T: Zeroable,
    {
      if data.is_empty() {
        let data = vec![T::zeroed()];
        StorageBufferReadOnlyDataView::create(device, &data)
      } else {
        StorageBufferReadOnlyDataView::create(device, data)
      }
    }
    // upload blas
    use bytemuck::cast_slice;
    let gpu_blas_meta_info = create_gpu_buffer(device, &blas_meta_info);
    let gpu_tri_bvh_root = create_gpu_buffer(device, &tri_bvh_root);
    // let gpu_box_bvh_root = create_gpu_buffer(device, &box_bvh_root);
    let gpu_tri_bvh_forest = create_gpu_buffer(device, &tri_bvh_forest);
    // let gpu_box_bvh_forest = create_gpu_buffer(device, &box_bvh_forest);
    let gpu_indices_redirect = create_gpu_buffer(device, &indices_redirect);
    let gpu_indices = create_gpu_buffer(device, &indices);
    let gpu_vertices = create_gpu_buffer(device, cast_slice(&vertices));
    // let gpu_boxes = create_gpu_buffer(device, &cast_slice(&boxes).to_vec());

    // build tlas
    let mut tlas_bvh_root = vec![];
    let mut tlas_bvh_forest = vec![];
    let mut tlas_data = vec![];
    let mut tlas_bounding = vec![];

    for tlas in &self.tlas_data {
      if let Some(tlas) = tlas {
        let bvh_start = tlas_bvh_forest.len() as u32;
        let primitive_start = tlas_data.len() as u32;

        let (tlas_bvh, tlas_traverse_next) =
          Self::build_tlas(tlas, &blas_box, &mut tlas_data, &mut tlas_bounding);
        let nodes = tlas_bvh
          .nodes
          .iter()
          .zip(tlas_traverse_next)
          .map(|(node, (hit, miss))| {
            flatten_bvh_to_gpu_node(node, hit, miss, bvh_start, primitive_start)
          });
        tlas_bvh_root.push(bvh_start);
        tlas_bvh_forest.extend(nodes);
      } else {
        tlas_bvh_root.push(INVALID_NEXT); // tested in bvh iter
      }
    }

    // upload tlas
    let gpu_tlas_bvh_root = create_gpu_buffer(device, &tlas_bvh_root);
    let gpu_tlas_bvh_forest = create_gpu_buffer(device, &tlas_bvh_forest);
    let gpu_tlas_data = create_gpu_buffer(device, &tlas_data);
    let gpu_tlas_bounding = create_gpu_buffer(device, &tlas_bounding);

    let cpu = NaiveSahBvhCpu {
      tlas_bvh_root,
      tlas_bvh_forest,
      tlas_data,
      tlas_bounding,
      blas_meta_info,
      tri_bvh_root,
      tri_bvh_forest,
      indices_redirect,
      indices,
      vertices,
    };
    // println!("{cpu:#?}");
    *cpu_data = Some(cpu);

    *gpu_data = Some(NaiveSahBvhGpu {
      tlas_bvh_root: gpu_tlas_bvh_root,
      tlas_bvh_forest: gpu_tlas_bvh_forest,
      tlas_data: gpu_tlas_data,
      tlas_bounding: gpu_tlas_bounding,
      blas_meta_info: gpu_blas_meta_info,
      tri_bvh_root: gpu_tri_bvh_root,
      tri_bvh_forest: gpu_tri_bvh_forest,
      indices_redirect: gpu_indices_redirect,
      indices: gpu_indices,
      vertices: gpu_vertices,
    });
  }
}

#[derive(Clone)]
pub struct NaiveSahBVHSystem {
  inner: Arc<RwLock<NaiveSahBVHSystemInner>>,
  device: GPUDevice,
}
struct NaiveSahBVHSystemInner {
  source: NaiveSahBvhSource,
  cpu_data: Option<NaiveSahBvhCpu>,
  gpu_data: Option<NaiveSahBvhGpu>,
}

impl NaiveSahBVHSystem {
  pub(crate) fn new(gpu: GPU) -> Self {
    Self {
      inner: Arc::new(RwLock::new(NaiveSahBVHSystemInner {
        source: Default::default(),
        cpu_data: None,
        gpu_data: None,
      })),
      device: gpu.device.clone(),
    }
  }

  fn get_or_build_gpu_data(&self) -> impl Deref<Target = NaiveSahBvhGpu> + '_ {
    let read = self.inner.read().unwrap();
    if read.gpu_data.is_some() {
      RwLockReadGuard::map(read, |g| g.gpu_data.as_ref().unwrap())
    } else {
      drop(read);

      let mut write = self.inner.write().unwrap();
      write.rebuild_acceleration_structures(&self.device);
      drop(write);

      let read = self.inner.read().unwrap();
      assert!(read.gpu_data.is_some());
      RwLockReadGuard::map(read, |g| g.gpu_data.as_ref().unwrap())
    }
  }
}
impl NaiveSahBVHSystemInner {
  fn invalidate(&mut self) {
    self.cpu_data = None;
    self.gpu_data = None;
  }
  fn rebuild_acceleration_structures(&mut self, device: &GPUDevice) {
    self
      .source
      .build(device, &mut self.cpu_data, &mut self.gpu_data);
  }
}

impl GPUAccelerationStructureSystemProvider for NaiveSahBVHSystem {
  fn create_comp_instance(&self) -> Box<dyn GPUAccelerationStructureSystemCompImplInstance> {
    let gpu = self.get_or_build_gpu_data();
    Box::new(gpu.clone())
  }

  // todo return instance ids? then TLAS device should store InstanceId
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> TlasHandle {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    let idx = inner.source.create_tlas(source);
    TlasHandle(idx)
  }

  fn delete_top_level_acceleration_structure(&self, handle: TlasHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    inner.source.delete_tlas(handle)
  }

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BlasHandle {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    let idx = inner.source.create_blas(source);
    BlasHandle(idx)
  }

  fn delete_bottom_level_acceleration_structure(&self, handle: BlasHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    inner.source.delete_blas(handle)
  }
}

const INVALID_NEXT: u32 = u32::MAX;
fn compute_bvh_next<B: BVHBounding>(flatten_nodes: &[FlattenBVHNode<B>]) -> Vec<(u32, u32)> {
  let mut result = vec![];
  let mut next_stack = vec![];

  for node in flatten_nodes {
    if next_stack.last().cloned() == Some(node.self_index as u32) {
      next_stack.pop();
    }
    let miss = next_stack.last().cloned().unwrap_or(INVALID_NEXT);
    let (hit, miss) =
      if let (Some(left), Some(right)) = (node.left_child_offset(), node.right_child_offset()) {
        let hit = left as u32;
        next_stack.push(right as u32);
        (hit, miss)
      } else {
        (miss, miss)
      };
    result.push((hit, miss));
  }
  result
}
