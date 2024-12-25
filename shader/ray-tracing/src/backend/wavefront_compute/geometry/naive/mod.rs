#[cfg(test)]
mod test;
#[cfg(test)]
pub(crate) use test::*;

mod blas;
mod flag;
mod traverse_cpu;
mod traverse_gpu;

use std::ops::{BitAnd, Deref};
use std::sync::{RwLock, RwLockReadGuard};

use blas::*;
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
  pub acceleration_structure_handle: u32, // blas id
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

#[derive(Default)]
struct NaiveSahBvhSource {
  blas_source: Vec<Option<Vec<BottomLevelAccelerationStructureBuildSource>>>,
  blas_pool: BuiltBlasPool,
  tlas_source: Vec<Option<Vec<TopLevelAccelerationStructureSourceInstance>>>,
  blas_free_list: Vec<usize>,
  tlas_free_list: Vec<usize>,
}

impl NaiveSahBvhSource {
  pub fn create_blas(&mut self, source: &[BottomLevelAccelerationStructureBuildSource]) -> u32 {
    let index = if let Some(index) = self.blas_free_list.pop() {
      assert!(self.blas_source[index].is_none());
      self.blas_source[index] = Some(source.to_vec());
      // println!("create blas reuse {index}");
      index
    } else {
      let index = self.blas_source.len();
      self.blas_source.push(Some(source.to_vec()));
      // println!("create blas new {index}");
      index
    };
    self.blas_pool.create(index, source);
    index as u32
  }
  pub fn create_tlas(&mut self, source: &[TopLevelAccelerationStructureSourceInstance]) -> u32 {
    let index = if let Some(index) = self.tlas_free_list.pop() {
      assert!(self.tlas_source[index].is_none());
      self.tlas_source[index] = Some(source.to_vec());
      // println!("create tlas reuse {index}");
      index
    } else {
      let index = self.tlas_source.len();
      self.tlas_source.push(Some(source.to_vec()));
      // println!("create tlas new {index}");
      index
    };
    index as u32
  }
  pub fn delete_blas(&mut self, handle: BlasHandle) {
    let index = handle.0 as usize;
    self.blas_pool.delete(index);
    if index == self.blas_source.len() - 1 {
      self.blas_source.pop();
      // println!("delete blas {index} shrink");
    } else {
      self.blas_source[index] = None;
      self.blas_free_list.push(index);
      // println!("delete blas {index} set none");
    }
  }
  pub fn delete_tlas(&mut self, handle: TlasHandle) {
    let index = handle.0 as usize;
    if index == self.tlas_source.len() - 1 {
      self.tlas_source.pop();
      // println!("delete tlas {index} shrink");
    } else {
      self.tlas_source[index] = None;
      self.tlas_free_list.push(index);
      // println!("delete tlas {index} set none");
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
    let (blas_data_cpu, blas_data_gpu) = self.blas_pool.get(device);

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

    // build tlas
    let mut tlas_bvh_root = vec![];
    let mut tlas_bvh_forest = vec![];
    let mut tlas_data = vec![];
    let mut tlas_bounding = vec![];

    for tlas in &self.tlas_source {
      if let Some(tlas) = tlas {
        let bvh_start = tlas_bvh_forest.len() as u32;
        let primitive_start = tlas_data.len() as u32;

        let (tlas_bvh, tlas_traverse_next) = Self::build_tlas(
          tlas,
          &blas_data_cpu.blas_bounding,
          &mut tlas_data,
          &mut tlas_bounding,
        );
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
    let gpu_tlas_bvh_root = create_gpu_buffer_non_empty(device, &tlas_bvh_root);
    let gpu_tlas_bvh_forest = create_gpu_buffer_non_empty(device, &tlas_bvh_forest);
    let gpu_tlas_data = create_gpu_buffer_non_empty(device, &tlas_data);
    let gpu_tlas_bounding = create_gpu_buffer_non_empty(device, &tlas_bounding);

    let cpu = NaiveSahBvhCpu {
      tlas_bvh_root,
      tlas_bvh_forest,
      tlas_data,
      tlas_bounding,
      blas_data: blas_data_cpu.clone(),
    };
    // println!("{cpu:#?}");
    *cpu_data = Some(cpu);

    *gpu_data = Some(NaiveSahBvhGpu {
      tlas_bvh_root: gpu_tlas_bvh_root,
      tlas_bvh_forest: gpu_tlas_bvh_forest,
      tlas_data: gpu_tlas_data,
      tlas_bounding: gpu_tlas_bounding,

      blas_data: blas_data_gpu.clone(),
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
    // inner.invalidate();
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

pub(crate) fn create_gpu_buffer_non_empty<T>(
  device: &GPUDevice,
  data: &Vec<T>,
) -> StorageBufferReadOnlyDataView<[T]>
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
