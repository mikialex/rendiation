use std::ops::{BitAnd, Deref, Range};
use std::sync::{RwLock, RwLockReadGuard};

use rendiation_geometry::Box3;
use rendiation_space_algorithm::bvh::*;
use rendiation_space_algorithm::utils::TreeBuildOption;

use crate::backend::wavefront_compute::geometry::{
  intersect_ray_triangle_cpu, intersect_ray_triangle_gpu, Ray,
};
use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
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
  pub box_root_range: Vec2<u32>,
}

#[derive(Default)]
struct NaiveSahBvhSource {
  blas_data: Vec<Option<Vec<BottomLevelAccelerationStructureBuildSource>>>,
  tlas_data: Vec<Option<TopLevelAccelerationStructureSourceInstance>>,
}

pub type TlasHandle = Range<u32>;
impl GPUAccelerationStructureInstanceProvider for Range<u32> {
  fn access_impl(&self) -> &dyn Any {
    self as &dyn Any
  }
}

impl NaiveSahBvhSource {
  pub fn create_blas(&mut self, source: &[BottomLevelAccelerationStructureBuildSource]) -> u32 {
    // TODO freelist
    let index = self.blas_data.len();
    self.blas_data.push(Some(source.to_vec()));
    index as u32
  }
  pub fn create_tlas(
    &mut self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> TlasHandle {
    // TODO freelist
    let start_index = self.tlas_data.len();
    for source in source {
      self.tlas_data.push(Some(*source));
    }
    start_index as u32..self.tlas_data.len() as u32
  }
  pub fn delete_blas(&mut self, i: u32) {
    // TODO freelist
    self.blas_data[i as usize] = None;
  }
  pub fn delete_tlas(&mut self, range: &TlasHandle) {
    // TODO freelist
    for i in range.start..range.end {
      self.tlas_data[i as usize] = None;
    }
  }

  // TODO incremental change
  /// returns (
  ///   blas_meta_info,
  ///   blas_box,
  ///   (tri_bvh, hit miss, index_offset, geometry_idx),
  ///   (box_bvh, hit miss, box_offset, geometry_idx),
  ///   indices,
  ///   vertices,
  ///   boxes
  /// )
  fn build_blas(
    blas_data: &[Option<Vec<BottomLevelAccelerationStructureBuildSource>>],
  ) -> (
    Vec<BlasMetaInfo>,
    Vec<Box3>,
    Vec<(FlattenBVH<Box3>, Vec<(u32, u32)>, u32, u32)>,
    Vec<(FlattenBVH<Box3>, Vec<(u32, u32)>, u32, u32)>,
    Vec<u32>,
    Vec<Vec3<f32>>,
    Vec<Vec3<f32>>,
  ) {
    let mut geometry_indices = vec![];
    let mut geometry_vertices = vec![];
    let mut geometry_boxes = vec![];

    let mut blas_meta_info = vec![];
    let mut blas_box = vec![];
    let mut tri_bvh = vec![];
    let mut box_bvh = vec![];

    for blas in blas_data {
      if blas.is_none() {
        blas_meta_info.push(BlasMetaInfo::zeroed());
        continue;
      }
      let blas = blas.as_ref().unwrap();

      let tri_start = tri_bvh.len();
      let box_start = box_bvh.len();

      // TODO par_iter
      for (geometry_idx, source) in blas.iter().enumerate() {
        let geometry_idx = geometry_idx as u32;
        let mut root_box = Box3::default();
        match source {
          BottomLevelAccelerationStructureBuildSource::Triangles { positions, indices } => {
            let index_start = geometry_indices.len() as u32;
            let vertex_start = geometry_vertices.len() as u32;

            geometry_vertices.extend_from_slice(positions);

            let boxes = indices.chunks_exact(3).map(|triangle| {
              triangle
                .iter()
                .map(|idx| positions[*idx as usize])
                .collect::<Box3>()
            });

            let option = TreeBuildOption {
              max_tree_depth: 50,
              bin_size: 10,
            };
            let mut sah = SAH::new(4);
            let bvh = FlattenBVH::new(boxes, &mut sah, &option);
            root_box.expand_by_other(bvh.nodes[0].bounding);
            let traverse_next = compute_bvh_next(&bvh.nodes);

            for primitive_idx in &bvh.sorted_primitive_index {
              geometry_indices.push(vertex_start + indices[primitive_idx * 3]);
              geometry_indices.push(vertex_start + indices[primitive_idx * 3 + 1]);
              geometry_indices.push(vertex_start + indices[primitive_idx * 3 + 2]);
            }

            tri_bvh.push((bvh, traverse_next, index_start, geometry_idx));
          }

          BottomLevelAccelerationStructureBuildSource::AABBs { aabbs } => {
            let boxes_start = geometry_indices.len() as u32;

            let boxes = aabbs.iter().map(|aabb| {
              let mut r = Box3::default();
              let point0 = vec3(aabb[0], aabb[1], aabb[2]);
              let point1 = vec3(aabb[3], aabb[4], aabb[5]);
              r.expand_by_point(point0);
              r.expand_by_point(point1);
              r
            });

            let option = TreeBuildOption {
              max_tree_depth: 50,
              bin_size: 10,
            };
            let mut sah = SAH::new(4);
            let bvh = FlattenBVH::new(boxes, &mut sah, &option);
            root_box.expand_by_other(bvh.nodes[0].bounding);
            let traverse_next = compute_bvh_next(&bvh.nodes);

            for primitive_idx in &bvh.sorted_primitive_index {
              let aabb = &aabbs[*primitive_idx];
              let point0 = vec3(aabb[0], aabb[1], aabb[2]);
              let point1 = vec3(aabb[3], aabb[4], aabb[5]);
              geometry_boxes.push(point0);
              geometry_boxes.push(point1);
            }

            box_bvh.push((bvh, traverse_next, boxes_start, geometry_idx));
          }
        }
        blas_box.push(root_box);
      }

      let triangle_end = tri_bvh.len();
      let box_end = box_bvh.len();

      blas_meta_info.push(BlasMetaInfo {
        tri_root_range: vec2(tri_start as u32, triangle_end as u32),
        box_root_range: vec2(box_start as u32, box_end as u32),
        ..Zeroable::zeroed()
      });
    }

    (
      blas_meta_info,
      blas_box,
      tri_bvh,
      box_bvh,
      geometry_indices,
      geometry_vertices,
      geometry_boxes,
    )
  }

  fn build_tlas(
    tlas_data: &mut [Option<TopLevelAccelerationStructureSourceInstance>],
    blas_box: &[Box3],
  ) -> (
    FlattenBVH<Box3>,
    Vec<(u32, u32)>,
    Vec<TopLevelAccelerationStructureSourceDeviceInstance>,
    Vec<TlasBounding>,
  ) {
    let mut tlas_bvh_aabb = vec![];
    let mut index_mapping = vec![]; // index_mapping[i++] = tlas_data[k]

    for (idx, tlas) in tlas_data.iter().enumerate() {
      if let Some(source) = tlas {
        let blas_idx = source.acceleration_structure_handle as usize;
        let aabb = blas_box[blas_idx].apply_matrix_into(source.transform);
        index_mapping.push(idx);
        tlas_bvh_aabb.push(aabb);
      }
    }

    let option = TreeBuildOption {
      max_tree_depth: 50,
      bin_size: 10,
    };
    let mut sah = SAH::new(4);
    let bvh = FlattenBVH::new(tlas_bvh_aabb.clone().into_iter(), &mut sah, &option);
    let traverse_next = compute_bvh_next(&bvh.nodes);

    let mut tlas_boundings = vec![];
    let mut tlas_items = vec![];

    for box_idx in &bvh.sorted_primitive_index {
      let aabb = tlas_bvh_aabb[*box_idx];
      let tlas_idx = index_mapping[*box_idx];
      let source = &tlas_data[tlas_idx].unwrap();

      let tlas_item = TopLevelAccelerationStructureSourceDeviceInstance {
        transform: source.transform,
        transform_inv: source.transform.inverse_or_identity(),
        instance_custom_index: source.instance_custom_index,
        instance_shader_binding_table_record_offset: source
          .instance_shader_binding_table_record_offset,
        flags: source.flags,
        acceleration_structure_handle: source.acceleration_structure_handle as u32,
        ..Zeroable::zeroed()
      };
      let tlas_bounding = TlasBounding {
        world_min: aabb.min,
        world_max: aabb.max,
        mask: source.mask,
        flags: source.flags,
        ..Zeroable::zeroed()
      };
      tlas_items.push(tlas_item);
      tlas_boundings.push(tlas_bounding);
    }

    (bvh, traverse_next, tlas_items, tlas_boundings)
  }

  pub fn build(
    &mut self,
    device: &GPUDevice,
    cpu_data: &mut Option<NaiveSahBvhCpu>,
    gpu_data: &mut Option<NaiveSahBvhGpu>,
  ) {
    // build blas
    let (blas_meta_info, blas_box, tri_bvh, box_bvh, indices, vertices, boxes) =
      Self::build_blas(&self.blas_data);
    fn flatten_bvh_to_gpu_node(
      node: &FlattenBVHNode<Box3>,
      hit: u32,
      miss: u32,
      offset: u32,
    ) -> DeviceBVHNode {
      DeviceBVHNode {
        aabb_min: node.bounding.min,
        aabb_max: node.bounding.max,
        hit_next: hit,
        miss_next: miss,
        content_range: vec2(
          node.primitive_range.start as u32 + offset,
          node.primitive_range.end as u32 + offset,
        ),
        ..Zeroable::zeroed()
      }
    }
    let mut tri_bvh_root = vec![];
    let mut box_bvh_root = vec![];
    let mut tri_bvh_forest = vec![];
    let mut box_bvh_forest = vec![];
    for (bvh, hit_miss, offset, geometry_idx) in tri_bvh {
      tri_bvh_root.push(vec3(tri_bvh_forest.len() as u32, geometry_idx, offset));
      let nodes = bvh
        .nodes
        .iter()
        .zip(hit_miss)
        .map(|(node, (hit, miss))| flatten_bvh_to_gpu_node(node, hit, miss, offset));
      tri_bvh_forest.extend(nodes);
    }
    for (bvh, hit_miss, offset, geometry_idx) in box_bvh {
      box_bvh_root.push(vec3(box_bvh_forest.len() as u32, geometry_idx, offset));
      let nodes = bvh
        .nodes
        .iter()
        .zip(hit_miss)
        .map(|(node, (hit, miss))| flatten_bvh_to_gpu_node(node, hit, miss, offset));
      box_bvh_forest.extend(nodes);
    }

    fn create_gpu_buffer<T>(device: &GPUDevice, data: &Vec<T>) -> StorageBufferReadOnlyDataView<[T]>
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
    let gpu_blas_meta_info = create_gpu_buffer(device, &blas_meta_info);
    let gpu_tri_bvh_root = create_gpu_buffer(device, &tri_bvh_root);
    let gpu_box_bvh_root = create_gpu_buffer(device, &box_bvh_root);
    let gpu_tri_bvh_forest = create_gpu_buffer(device, &tri_bvh_forest);
    let gpu_box_bvh_forest = create_gpu_buffer(device, &box_bvh_forest);
    let gpu_indices = create_gpu_buffer(device, &indices);
    let gpu_vertices = create_gpu_buffer(device, &vertices);
    let gpu_boxes = create_gpu_buffer(device, &boxes);

    // build tlas
    let mut tlas_bvh_forest = vec![];
    let (tlas_bvh, tlas_traverse_next, tlas_data, tlas_bounding) =
      Self::build_tlas(&mut self.tlas_data, &blas_box);
    {
      let nodes = tlas_bvh
        .nodes
        .iter()
        .zip(tlas_traverse_next)
        .map(|(node, (hit, miss))| flatten_bvh_to_gpu_node(node, hit, miss, 0));
      tlas_bvh_forest.extend(nodes);
    }

    // upload tlas
    let gpu_tlas_bvh_forest = create_gpu_buffer(device, &tlas_bvh_forest);
    let gpu_tlas_data = create_gpu_buffer(device, &tlas_data);
    let gpu_tlas_bounding = create_gpu_buffer(device, &tlas_bounding);

    let cpu = NaiveSahBvhCpu {
      tlas_bvh_forest,
      tlas_data,
      tlas_bounding,
      blas_meta_info,
      tri_bvh_root,
      box_bvh_root,
      tri_bvh_forest,
      box_bvh_forest,
      indices,
      vertices,
      boxes,
    };
    // println!("{cpu:#?}");
    *cpu_data = Some(cpu);

    *gpu_data = Some(NaiveSahBvhGpu {
      tlas_bvh_forest: gpu_tlas_bvh_forest,
      tlas_data: gpu_tlas_data,
      tlas_bounding: gpu_tlas_bounding,
      blas_meta_info: gpu_blas_meta_info,
      tri_bvh_root: gpu_tri_bvh_root,
      box_bvh_root: gpu_box_bvh_root,
      tri_bvh_forest: gpu_tri_bvh_forest,
      box_bvh_forest: gpu_box_bvh_forest,
      indices: gpu_indices,
      vertices: gpu_vertices,
      boxes: gpu_boxes,
    });
  }
}

#[derive(Debug)]
struct NaiveSahBvhCpu {
  // global bvh, root at 0, content_range to index tlas_data/tlas_bounding
  tlas_bvh_forest: Vec<DeviceBVHNode>,
  // acceleration_structure_handle to index blas_meta_info
  tlas_data: Vec<TopLevelAccelerationStructureSourceDeviceInstance>,
  tlas_bounding: Vec<TlasBounding>,

  // tri_range to index tri_bvh_root, box_range to index box_bvh_root
  blas_meta_info: Vec<BlasMetaInfo>,
  // vec3(tri_bvh_forest root_idx, geometry_idx, primitive_start)
  tri_bvh_root: Vec<Vec3<u32>>,
  // vec3(box_bvh_forest root_idx, geometry_idx, primitive_start)
  box_bvh_root: Vec<Vec3<u32>>,
  // content range to index indices
  tri_bvh_forest: Vec<DeviceBVHNode>,
  // content range to index boxes
  box_bvh_forest: Vec<DeviceBVHNode>,

  indices: Vec<u32>,
  vertices: Vec<Vec3<f32>>,
  boxes: Vec<Vec3<f32>>,
}
#[derive(Clone)]
struct NaiveSahBvhGpu {
  // global bvh, root at 0, content_range to index tlas_data/tlas_bounding
  tlas_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  // acceleration_structure_handle to index blas_meta_info
  tlas_data: StorageBufferReadOnlyDataView<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  tlas_bounding: StorageBufferReadOnlyDataView<[TlasBounding]>,

  // tri_range to index tri_bvh_root, box_range to index box_bvh_root
  blas_meta_info: StorageBufferReadOnlyDataView<[BlasMetaInfo]>,
  // vec3(tri_bvh_forest root_idx, geometry_idx, primitive_start)
  tri_bvh_root: StorageBufferReadOnlyDataView<[Vec3<u32>]>,
  // vec3(box_bvh_forest root_idx, geometry_idx, primitive_start)
  box_bvh_root: StorageBufferReadOnlyDataView<[Vec3<u32>]>,
  // content range to index indices
  tri_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  // content range to index boxes
  box_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,

  indices: StorageBufferReadOnlyDataView<[u32]>,
  vertices: StorageBufferReadOnlyDataView<[Vec3<f32>]>,
  boxes: StorageBufferReadOnlyDataView<[Vec3<f32>]>,
}

#[derive(Clone)]
struct NaiveSahBVHSystem {
  inner: Arc<RwLock<NaiveSahBVHSystemInner>>,
  device: GPUDevice,
}
struct NaiveSahBVHSystemInner {
  source: NaiveSahBvhSource,
  cpu_data: Option<NaiveSahBvhCpu>,
  gpu_data: Option<NaiveSahBvhGpu>,
}

impl NaiveSahBVHSystem {
  async fn new() -> Self {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    Self {
      inner: Arc::new(RwLock::new(NaiveSahBVHSystemInner {
        source: Default::default(),
        cpu_data: None,
        gpu_data: None,
      })),
      device: gpu.device,
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

impl GPUAccelerationStructureSystemCompImplInstance for NaiveSahBvhGpu {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable> {
    let tlas_bvh_forest = compute_cx.bind_by(&self.tlas_bvh_forest);
    let tlas_data = compute_cx.bind_by(&self.tlas_data);
    let tlas_bounding = compute_cx.bind_by(&self.tlas_bounding);
    let blas_meta_info = compute_cx.bind_by(&self.blas_meta_info);
    let tri_bvh_root = compute_cx.bind_by(&self.tri_bvh_root);
    let box_bvh_root = compute_cx.bind_by(&self.box_bvh_root);
    let tri_bvh_forest = compute_cx.bind_by(&self.tri_bvh_forest);
    let box_bvh_forest = compute_cx.bind_by(&self.box_bvh_forest);
    let indices = compute_cx.bind_by(&self.indices);
    let vertices = compute_cx.bind_by(&self.vertices);
    let boxes = compute_cx.bind_by(&self.boxes);

    let instance = NaiveSahBVHInvocationInstance {
      tlas_bvh_forest,
      tlas_data,
      tlas_bounding,
      blas_meta_info,
      tri_bvh_root,
      box_bvh_root,
      tri_bvh_forest,
      box_bvh_forest,
      indices,
      vertices,
      boxes,
    };

    Box::new(instance)
  }

  fn bind_pass(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.tlas_bvh_forest);
    builder.bind(&self.tlas_data);
    builder.bind(&self.blas_meta_info);
    builder.bind(&self.tri_bvh_root);
    builder.bind(&self.box_bvh_root);
    builder.bind(&self.tri_bvh_forest);
    builder.bind(&self.box_bvh_forest);
    builder.bind(&self.indices);
    builder.bind(&self.vertices);
    builder.bind(&self.boxes);
  }
}

pub struct NaiveSahBVHInvocationInstance {
  tlas_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  tlas_bounding: ReadOnlyStorageNode<[TlasBounding]>,
  blas_meta_info: ReadOnlyStorageNode<[BlasMetaInfo]>,
  tri_bvh_root: ReadOnlyStorageNode<[Vec3<u32>]>,
  box_bvh_root: ReadOnlyStorageNode<[Vec3<u32>]>,
  tri_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  box_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  indices: ReadOnlyStorageNode<[u32]>,
  vertices: ReadOnlyStorageNode<[Vec3<f32>]>,
  boxes: ReadOnlyStorageNode<[Vec3<f32>]>,
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
fn traverse_bvh_cpu(
  root_idx: u32,
  bvh: &[DeviceBVHNode],
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: Vec2<f32>,
  mut hit_leaf: impl FnMut(&DeviceBVHNode),
) {
  let mut curr = root_idx;
  while curr != INVALID_NEXT {
    let node = &bvh[curr as usize];
    let next = if intersect_ray_aabb_cpu(
      ray_origin,
      ray_direction,
      ray_range,
      node.aabb_min,
      node.aabb_max,
    ) {
      if node.hit_next == node.miss_next {
        // is leaf
        hit_leaf(node);
      }
      node.hit_next
    } else {
      node.miss_next
    };
    curr = next;
  }
}

struct TraverseBvhIteratorGpu {
  bvh: ReadOnlyStorageNode<[DeviceBVHNode]>,
  ray: Node<Ray>,
  node_idx: LocalVarNode<u32>,
}
impl ShaderIterator for TraverseBvhIteratorGpu {
  type Item = Node<Vec2<u32>>; // node content range
  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let item = zeroed_val().make_local_var();

    loop_by(|loop_cx| {
      let idx = self.node_idx.load();
      if_by(idx.equals(val(INVALID_NEXT)), || loop_cx.do_break());
      let node = self.bvh.index(idx).load().expand();
      let hit_aabb = intersect_ray_aabb_gpu(self.ray, node.aabb_min, node.aabb_max);

      if_by(hit_aabb, || {
        let is_leaf = node.hit_next.equals(node.miss_next);
        if_by(is_leaf, || {
          has_next.store(val(true));
          item.store(node.content_range);
          self.node_idx.store(node.hit_next);
          loop_cx.do_break();
        })
        .else_by(|| {
          self.node_idx.store(node.miss_next);
        });
      });
    });

    (has_next.load(), item.load())
  }
}

/// returns iterator item = tlas_data idx
fn traverse_tlas_gpu(
  root: Node<u32>,
  bvh: ReadOnlyStorageNode<[DeviceBVHNode]>,
  tlas_bounding: ReadOnlyStorageNode<[TlasBounding]>,
  ray: Node<Ray>,
) -> impl ShaderIterator<Item = Node<u32>> {
  let bvh_iter = TraverseBvhIteratorGpu {
    bvh,
    ray,
    node_idx: root.make_local_var(),
  };
  let iter = bvh_iter.flat_map(ForRange::new);

  iter.filter_map(move |tlas_idx: Node<u32>| {
    let tlas_bounding_pack = tlas_bounding.index(tlas_idx).load();
    let tlas_bounding = tlas_bounding_pack.expand();
    let hit_tlas = intersect_ray_aabb_gpu(ray, tlas_bounding.world_min, tlas_bounding.world_max);

    let ray = ray.expand();
    let pass_mask = ray.mask.bitand(tlas_bounding.mask).not_equals(val(0));

    // TODO handle flags?
    let hit = hit_tlas.and(pass_mask);

    (hit, tlas_idx)
  })
}

struct NaiveIntersectReporter<'a> {
  launch_info: RayLaunchInfo,
  world_ray: WorldRayInfo,
  hit_ctx: HitCtxInfo,
  closest_hit_ctx_info: &'a HitCtxInfoRegister,
  closest_hit_info: &'a HitInfoRegister,
  any_hit: &'a dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
}
impl<'a> IntersectionReporter for NaiveIntersectReporter<'a> {
  fn report_intersection(&self, hit_t: Node<f32>, hit_kind: Node<u32>) -> Node<bool> {
    let r = val(false).make_local_var();
    if_by(
      hit_t.less_than(self.closest_hit_info.hit_distance.load()),
      || {
        let behavior = (self.any_hit)(&RayAnyHitCtx {
          launch_info: self.launch_info,
          world_ray: self.world_ray,
          hit_ctx: self.hit_ctx,
          hit: HitInfo {
            hit_kind,
            hit_distance: hit_t,
          },
        });

        if_by(behavior.equals(val(IGNORE_THIS_INTERSECTION)), || {
          // TODO ignore?
        })
        .else_if(behavior.equals(val(TERMINATE_TRAVERSE)), || {
          // TODO terminate
        })
        .else_by(|| {
          // hit! update closest
          self.closest_hit_ctx_info.store(&self.hit_ctx);
          self.closest_hit_info.test_and_store(&HitInfo {
            hit_kind,
            hit_distance: hit_t,
          });
          // TODO update ray range max
          r.store(val(true));
        });
      },
    );
    r.load()
  }
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
struct RayBlas {
  pub ray: Ray,
  pub blas: BlasMetaInfo,
  pub tlas_idx: u32,
  pub distance_scaling: f32,
}

fn iterate_tlas_blas_gpu(
  tlas_iter: impl ShaderIterator<Item = Node<u32>>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  blas_data: ReadOnlyStorageNode<[BlasMetaInfo]>,
  ray: Node<Ray>,
) -> impl ShaderIterator<Item = Node<RayBlas>> {
  tlas_iter.map(move |idx: Node<u32>| {
    let ray = ray.expand();
    let tlas_data = tlas_data.index(idx).load().expand();

    // transform ray to blas space
    // TODO check det < 0, invert cull flag?
    let blas_ray_origin = tlas_data.transform_inv * (ray.origin, val(1.)).into();
    let blas_ray_origin = blas_ray_origin.xyz() / blas_ray_origin.w().splat();
    let blas_ray_direction = tlas_data.transform_inv.shrink_to_3() * ray.direction;
    let distance_scaling = blas_ray_direction.length();
    let blas_ray_range = ray.range * distance_scaling;
    let blas_ray_direction = blas_ray_direction.normalize();
    let blas_ray = Ray::construct(RayShaderAPIInstance {
      origin: blas_ray_origin,
      flags: ray.flags,
      direction: blas_ray_direction,
      mask: ray.mask,
      range: blas_ray_range,
    });

    let blas_idx = tlas_data.acceleration_structure_handle;
    let blas_data = blas_data.index(blas_idx).load();

    RayBlas::construct(RayBlasShaderAPIInstance {
      ray: blas_ray,
      blas: blas_data,
      tlas_idx: idx,
      distance_scaling,
    })
  })
}

fn intersect_blas_gpu(
  blas_iter: impl ShaderIterator<Item = Node<RayBlas>>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  tri_bvh_root: ReadOnlyStorageNode<[Vec3<u32>]>,
  tri_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  box_bvh_root: ReadOnlyStorageNode<[Vec3<u32>]>,
  box_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  indices: ReadOnlyStorageNode<[u32]>,
  vertices: ReadOnlyStorageNode<[Vec3<f32>]>,
  boxes: ReadOnlyStorageNode<[Vec3<f32>]>,

  intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
  any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,

  launch_info: RayLaunchInfo,
  world_ray: WorldRayInfo,
  closest_hit_ctx_info: &HitCtxInfoRegister,
  closest_hit_info: &HitInfoRegister,
) {
  blas_iter.for_each(|ray_blas, _cx| {
    let ray_blas = ray_blas.expand();
    let ray = ray_blas.ray;
    let blas = ray_blas.blas.expand();

    ForRange::new(blas.tri_root_range).for_each(move |tri_root_idx, _cx| {
      let ray_blas = ray_blas;
      let ray = ray;
      let geometry = tri_bvh_root.index(tri_root_idx).load();
      let root = geometry.x();
      let geometry_id = geometry.y();
      let primitive_start = geometry.z();

      let bvh_iter = TraverseBvhIteratorGpu {
        bvh: tri_bvh_forest,
        ray,
        node_idx: root.make_local_var(),
      };
      let iter = bvh_iter.flat_map(ForRange::new); // triangle index

      let ray = ray.expand();
      iter.for_each(move |tri_idx, _cx| {
        let start = tri_idx * val(3);
        let v0 = vertices.index(indices.index(start).load()).load();
        let v1 = vertices.index(indices.index(start + val(1)).load()).load();
        let v2 = vertices.index(indices.index(start + val(2)).load()).load();
        // returns (hit ? 1 : 0, distance, u, v)
        let result = intersect_ray_triangle_gpu(ray.origin, ray.direction, ray.range, v0, v1, v2);
        let hit = result.x().equals(val(0.));
        if_by(hit, move || {
          let world_distance = result.y() / ray_blas.distance_scaling;
          // TODO load tlas on every hit? protect with a bool?
          let tlas = tlas_data.index(ray_blas.tlas_idx).load().expand();

          let hit_ctx = HitCtxInfo {
            primitive_id: tri_idx - primitive_start, // store tri offset in tri_bvh_root
            instance_id: ray_blas.tlas_idx, // TODO not exactly instance id, deleted tlas are skipped
            instance_sbt_offset: tlas.instance_shader_binding_table_record_offset,
            instance_custom_id: tlas.instance_custom_index,
            geometry_id,
            object_to_world: tlas.transform_inv,
            world_to_object: tlas.transform,
            object_space_ray: ShaderRay {
              origin: ray.origin,
              direction: ray.direction,
            },
          };

          let intersect_ctx = RayIntersectCtx {
            launch_info,
            world_ray,
            hit_ctx,
          };
          intersect(&intersect_ctx, &NaiveIntersectReporter {
            launch_info,
            world_ray,
            hit_ctx,
            closest_hit_ctx_info,
            closest_hit_info,
            any_hit,
          });

          // intersect will invoke any_hit and then update closest_hit
          // TODO update intersect range to optimize
        });
      });
    });

    // ForRange::new(blas.box_root_range).for_each(|box_root_idx, _cx| {
    //   let geometry = box_bvh_root.index(box_root_idx).load();
    //   let root = geometry.x();
    //   let geometry_id = geometry.y();
    //
    //   let bvh_iter = TraverseBvhIteratorGpu {
    //     bvh: box_bvh_forest,
    //     ray,
    //     node_idx: root.make_local_var(),
    //   };
    //   let iter = bvh_iter.flat_map(ForRange::new); // box index
    //
    //   iter.for_each(|box_idx, _cx| {
    //     let start = box_idx * val(2);
    //     let min = boxes.index(indices.index(start).load()).load();
    //     let max = boxes.index(indices.index(start + val(1)).load()).load();
    //
    //     let hit = intersect_ray_aabb_gpu(ray, min, max);
    //     if_by(hit, || {
    //       // TODO call intersection with anyhit, remember distance_scaling
    //     });
    //   });
    // });
  });
}

impl NaiveSahBvhCpu {
  fn traverse(
    &self,
    ray: &mut ShaderRayTraceCallStoragePayload,
    any_hit: &mut dyn FnMut(u32, u32, f32, Vec3<f32>), /* geometry_idx, primitive_idx, distance, hit_position // TODO use ctx */
  ) {
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

        // traverse blas bvh
        // TODO prepare intersect ctx, anyhit ctx
        let blas_ray_origin = tlas_data.transform_inv * ray.ray_origin;
        let blas_ray_direction = ray
          .ray_direction
          .transform_direction(tlas_data.transform_inv)
          .value;

        let distance_scaling = (tlas_data.transform_inv.to_mat3() * ray.ray_direction).length();
        let blas_ray_range = ray.range * distance_scaling;

        // TODO triangle related flags
        let blas_meta_info = &self.blas_meta_info[blas_idx as usize];
        for tri_root_index in blas_meta_info.tri_root_range.x..blas_meta_info.tri_root_range.y {
          let idx = self.tri_bvh_root[tri_root_index as usize];
          let blas_root_idx = idx.x;
          let geometry_idx = idx.y;
          let primitive_offset = idx.z;

          let tri_iter = TraverseBvhIteratorCpu {
            bvh: &self.tri_bvh_forest,
            ray_origin: blas_ray_origin,
            ray_direction: blas_ray_direction,
            ray_range: blas_ray_range,
            curr_idx: blas_root_idx,
          };

          for hit_idx in tri_iter {
            let node = &self.tri_bvh_forest[hit_idx as usize];

            // intersect triangles
            let indices =
              &self.indices[node.content_range.x as usize * 3..node.content_range.y as usize * 3];
            for (primitive_idx, triangle) in indices.chunks_exact(3).enumerate() {
              let primitive_idx = primitive_idx as u32 - primitive_offset;
              let v0 = self.vertices[triangle[0] as usize];
              let v1 = self.vertices[triangle[1] as usize];
              let v2 = self.vertices[triangle[2] as usize];
              // vec4(hit, distance, u, v)
              let intersection = intersect_ray_triangle_cpu(
                blas_ray_origin,
                blas_ray_direction,
                blas_ray_range,
                v0,
                v1,
                v2,
                // TODO triangle related flags
              );

              if intersection[0] > 0. {
                let distance = intersection[1] / distance_scaling;
                let p = blas_ray_origin + distance * blas_ray_direction;
                // println!("hit {p:?}");
                any_hit(geometry_idx, primitive_idx, distance, p);
                // TODO call anyhit
                // TODO modify range after hit
              }
            }
          }
        }

        // TODO check box related flags
        for box_root_index in blas_meta_info.box_root_range.x..blas_meta_info.box_root_range.y {
          let idx = self.box_bvh_root[box_root_index as usize];
          let blas_root_idx = idx.x;
          let geometry_idx = idx.y;

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
                // TODO call intersect, then anyhit
                // TODO modify range after hit
              }
            }
          }
        }
      }
    }
  }
}

struct HitCtxInfoRegister {
  pub primitive_id: LocalVarNode<u32>,
  pub instance_id: LocalVarNode<u32>,
  pub instance_sbt_offset: LocalVarNode<u32>,
  pub instance_custom_id: LocalVarNode<u32>,
  pub geometry_id: LocalVarNode<u32>,
  pub object_to_world: LocalVarNode<Mat4<f32>>,
  pub world_to_object: LocalVarNode<Mat4<f32>>,
  pub object_space_ray_origin: LocalVarNode<Vec3<f32>>,
  pub object_space_ray_direction: LocalVarNode<Vec3<f32>>,
}
impl HitCtxInfoRegister {
  fn store(&self, source: &HitCtxInfo) {
    self.primitive_id.store(source.primitive_id);
    self.instance_id.store(source.instance_id);
    self.instance_sbt_offset.store(source.instance_sbt_offset);
    self.instance_custom_id.store(source.instance_custom_id);
    self.geometry_id.store(source.geometry_id);
    self.object_to_world.store(source.object_to_world);
    self.world_to_object.store(source.world_to_object);
    self
      .object_space_ray_origin
      .store(source.object_space_ray.origin);
    self
      .object_space_ray_direction
      .store(source.object_space_ray.direction);
  }
}
struct HitInfoRegister {
  pub any_hit: LocalVarNode<bool>,
  pub hit_kind: LocalVarNode<u32>,
  pub hit_distance: LocalVarNode<f32>,
}
impl HitInfoRegister {
  fn test_and_store(&self, source: &HitInfo) {
    if_by(
      source.hit_distance.less_than(self.hit_distance.load()),
      || {
        self.any_hit.store(val(true));
        self.hit_kind.store(source.hit_kind);
        self.hit_distance.store(source.hit_distance);
      },
    );
  }
}

impl GPUAccelerationStructureSystemCompImplInvocationTraversable for NaiveSahBVHInvocationInstance {
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<RayClosestHitCtx> {
    let ray = Ray::construct(RayShaderAPIInstance {
      origin: trace_payload.ray_origin,
      flags: trace_payload.ray_flags,
      direction: trace_payload.ray_direction,
      mask: trace_payload.cull_mask,
      range: trace_payload.range,
    });

    let tlas_idx_iter = traverse_tlas_gpu(val(0), self.tlas_bvh_forest, self.tlas_bounding, ray);

    let blas_iter = iterate_tlas_blas_gpu(tlas_idx_iter, self.tlas_data, self.blas_meta_info, ray);

    // construct ctx
    let launch_info = RayLaunchInfo {
      launch_id: val(vec3(0, 0, 0)),   // TODO
      launch_size: val(vec3(0, 0, 0)), // TODO
    };
    let world_ray = WorldRayInfo {
      world_ray: ShaderRay {
        origin: trace_payload.ray_origin,
        direction: trace_payload.ray_direction,
      },
      ray_range: ShaderRayRange {
        min: trace_payload.range.x(),
        max: trace_payload.range.y(),
      },
      ray_flags: trace_payload.ray_flags,
    };

    let hit_ctx_info_reg = HitCtxInfoRegister {
      primitive_id: val(0).make_local_var(),
      instance_id: val(0).make_local_var(),
      instance_sbt_offset: val(0).make_local_var(),
      instance_custom_id: val(0).make_local_var(),
      geometry_id: val(0).make_local_var(),
      object_to_world: val(Mat4::identity()).make_local_var(),
      world_to_object: val(Mat4::identity()).make_local_var(),
      object_space_ray_origin: val(vec3(0., 0., 0.)).make_local_var(),
      object_space_ray_direction: val(vec3(0., 0., 0.)).make_local_var(),
    };
    let hit_ctx_info = HitCtxInfo {
      primitive_id: hit_ctx_info_reg.primitive_id.load(),
      instance_id: hit_ctx_info_reg.instance_id.load(),
      instance_sbt_offset: hit_ctx_info_reg.instance_sbt_offset.load(),
      instance_custom_id: hit_ctx_info_reg.instance_custom_id.load(),
      geometry_id: hit_ctx_info_reg.geometry_id.load(),
      object_to_world: hit_ctx_info_reg.object_to_world.load(),
      world_to_object: hit_ctx_info_reg.world_to_object.load(),
      object_space_ray: ShaderRay {
        origin: hit_ctx_info_reg.object_space_ray_origin.load(),
        direction: hit_ctx_info_reg.object_space_ray_direction.load(),
      },
    };

    let hit_info_reg = HitInfoRegister {
      any_hit: val(false).make_local_var(),
      hit_kind: val(0).make_local_var(),
      hit_distance: world_ray.ray_range.max.make_local_var(),
    };
    let hit_info = HitInfo {
      hit_kind: hit_info_reg.hit_kind.load(),
      hit_distance: hit_info_reg.hit_distance.load(),
    };

    intersect_blas_gpu(
      blas_iter,
      self.tlas_data,
      self.tri_bvh_root,
      self.tri_bvh_forest,
      self.box_bvh_root,
      self.box_bvh_forest,
      self.indices,
      self.vertices,
      self.boxes,
      intersect,
      any_hit,
      launch_info,
      world_ray,
      &hit_ctx_info_reg, // output
      &hit_info_reg,     // output
    );

    DeviceOption {
      is_some: hit_info_reg.any_hit.load(),
      payload: RayClosestHitCtx {
        launch_info,
        world_ray,
        hit_ctx: hit_ctx_info,
        hit: hit_info,
      },
    }
  }
}

impl GPUAccelerationStructureSystemProvider for NaiveSahBVHSystem {
  fn create_comp_instance(&self) -> Box<dyn GPUAccelerationStructureSystemCompImplInstance> {
    let gpu = self.get_or_build_gpu_data();
    Box::new(gpu.clone())
  }

  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureInstanceProvider> {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    Box::new(inner.source.create_tlas(source))
  }

  fn delete_top_level_acceleration_structure(
    &self,
    id: Box<dyn GPUAccelerationStructureInstanceProvider>,
  ) {
    let range: &TlasHandle = id.access_impl().downcast_ref().unwrap();
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    inner.source.delete_tlas(range);
  }

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BottomLevelAccelerationStructureHandle {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    let index = inner.source.create_blas(source);
    BottomLevelAccelerationStructureHandle(index)
  }

  fn delete_bottom_level_acceleration_structure(&self, id: BottomLevelAccelerationStructureHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.invalidate();
    inner.source.delete_blas(id.0)
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

#[test]
fn test_cpu_triangle() {
  const W: usize = 256;
  const H: usize = 256;
  const FAR: f32 = 100.;
  // const GEOMETRY_IDX_MAX: u32 = 1;
  const PRIMITIVE_IDX_MAX: u32 = 12;

  #[rustfmt::skip]
  const CUBE_POSITION: [f32; 72] = [
       0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5, -0.5,  0.5,  0.5, -0.5,  0.5, // v0,v1,v2,v3 (front)
       0.5,  0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5, -0.5,  0.5,  0.5, -0.5, // v0,v3,v4,v5 (right)
       0.5,  0.5,  0.5,  0.5,  0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5,  0.5, // v0,v5,v6,v1 (top)
      -0.5,  0.5,  0.5, -0.5,  0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, // v1,v6,v7,v2 (left)
      -0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5, -0.5,  0.5, -0.5, -0.5,  0.5, // v7,v4,v3,v2 (bottom)
       0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, -0.5,  0.5,  0.5, -0.5, // v4,v7,v6,v5 (back)
  ];
  #[rustfmt::skip]
  const CUBE_INDEX: [u16; 36] = [
       0, 1, 2,   2, 3, 0,    // v0-v1-v2, v2-v3-v0 (front)
       4, 5, 6,   6, 7, 4,    // v0-v3-v4, v4-v5-v0 (right)
       8, 9,10,  10,11, 8,    // v0-v5-v6, v6-v1-v0 (top)
      12,13,14,  14,15,12,    // v1-v6-v7, v7-v2-v1 (left)
      16,17,18,  18,19,16,    // v7-v4-v3, v3-v2-v7 (bottom)
      20,21,22,  22,23,20,    // v4-v7-v6, v6-v5-v4 (back)
  ];

  let mut system = futures::executor::block_on(NaiveSahBVHSystem::new());
  let blas_handle = system.create_bottom_level_acceleration_structure(&[
    BottomLevelAccelerationStructureBuildSource::Triangles {
      positions: CUBE_POSITION
        .chunks_exact(3)
        .map(|abc| vec3(abc[0], abc[1], abc[2]))
        .collect(),
      indices: CUBE_INDEX.map(|i| i as u32).into_iter().collect(),
    },
  ]);

  fn add_tlas(
    system: &mut NaiveSahBVHSystem,
    transform: Mat4<f32>,
    blas_handle: &BottomLevelAccelerationStructureHandle,
  ) -> Box<dyn GPUAccelerationStructureInstanceProvider> {
    system.create_top_level_acceleration_structure(&[TopLevelAccelerationStructureSourceInstance {
      transform,
      instance_custom_index: 0,
      mask: u32::MAX,
      instance_shader_binding_table_record_offset: 0,
      flags: 0,
      acceleration_structure_handle: blas_handle.0 as u64,
    }])
  }
  for i in -2..=2 {
    for j in -2..=2 {
      add_tlas(
        &mut system,
        Mat4::translate((i as f32 * 1.5, j as f32 * 1.5, -10.)),
        &blas_handle,
      );
    }
  }
  add_tlas(
    &mut system,
    Mat4::translate((0., 4.5, -10.)) * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas(
    &mut system,
    Mat4::translate((0., -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas(
    &mut system,
    Mat4::translate((4.5, -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI * 0.5)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas(
    &mut system,
    Mat4::translate((-4.5, -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI * -0.5)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );

  let _ = system.get_or_build_gpu_data(); // trigger build
  let inner = system.inner.read().unwrap();
  let cpu_data = inner.cpu_data.as_ref().unwrap();

  let mut payload = ShaderRayTraceCallStoragePayload::zeroed();
  payload.cull_mask = u32::MAX;
  payload.range = vec2(0., FAR);

  let mut out = Box::new([[(FAR, 0); W]; H]);

  for j in 0..H {
    // println!("{j}");
    for i in 0..W {
      let x = (i as f32 + 0.5) / W as f32 * 2. - 1.;
      let y = 1. - (j as f32 + 0.5) / H as f32 * 2.;
      let origin = vec3(0., 0., 0.);
      let target = vec3(x, y, -1.); // fov = 90 deg
      let direction = (target - origin).normalize();
      // TODO pass &mut distance to traverse to optimize

      payload.ray_origin = origin;
      payload.ray_direction = direction;
      cpu_data.traverse(
        &mut payload,
        &mut |_geometry_id, primitive_id, distance, _position| {
          let (d, id) = &mut out[j][i];
          if distance < *d {
            *d = distance;
            *id = primitive_id + 1;
          }
        },
      );
    }
  }

  let mut file = format!("P2\n{W} {H}\n{PRIMITIVE_IDX_MAX}\n");
  for j in 0..H {
    file.push_str(out[j].map(|(_, id)| format!("{id}")).join(" ").as_str());
    file.push('\n');
  }
  std::fs::write("trace_cpu.pbm", file).unwrap();
}
