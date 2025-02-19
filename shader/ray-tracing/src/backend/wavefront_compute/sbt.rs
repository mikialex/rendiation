use crate::*;

pub struct ShaderBindingTableInfo {
  pub(crate) sys: ShaderBindingTableDeviceInfo,
  pub(crate) self_idx: u32,
  pub(crate) ray_stride: u32,
}

// todo support resize
// todo, fix correctly mapping to task index
impl ShaderBindingTableProvider for ShaderBindingTableInfo {
  fn config_ray_generation(&mut self, s: ShaderHandle) {
    let mut sys = self.sys.inner.write();
    let ray_gen_start = sys.meta.get(self.self_idx).unwrap().gen_start;
    sys.ray_gen.set_value(ray_gen_start, s.0).unwrap();
  }

  fn config_hit_group(
    &mut self,
    geometry_idx: u32,
    tlas_offset: u32,
    ray_ty_idx: u32,
    hit_group: HitGroupShaderRecord,
  ) {
    let mut sys = self.sys.inner.write();
    let hit_group_start = sys.meta.get(self.self_idx).unwrap().hit_group_start;
    sys
      .ray_hit
      .set_value(
        hit_group_start + ray_ty_idx + geometry_idx * self.ray_stride + tlas_offset,
        DeviceHitGroupShaderRecord {
          closest_hit: hit_group.closest_hit.map(|v| v.0).unwrap_or(u32::MAX),
          any_hit: hit_group.any_hit.map(|v| v.0).unwrap_or(u32::MAX),
          intersection: hit_group.intersection.map(|v| v.0).unwrap_or(u32::MAX),
          ..Zeroable::zeroed()
        },
      )
      .unwrap();
  }

  fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle) {
    let mut sys = self.sys.inner.write();
    let miss_start = sys.meta.get(self.self_idx).unwrap().miss_start;
    sys
      .ray_miss
      .set_value(miss_start + ray_ty_idx, s.0)
      .unwrap();
  }
  fn access_impl(&self) -> &dyn Any {
    self
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, PartialEq, StorageNodePtrAccess)]
pub struct DeviceSBTTableMeta {
  pub hit_group_start: u32,
  pub miss_start: u32,
  pub gen_start: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, StorageNodePtrAccess)]
pub struct DeviceHitGroupShaderRecord {
  pub closest_hit: u32,
  pub any_hit: u32,
  pub intersection: u32,
}

#[derive(Clone)]
pub struct ShaderBindingTableDeviceInfo {
  inner: Arc<RwLock<ShaderBindingTableDeviceInfoImpl>>,
}

pub struct ShaderBindingTableDeviceInfoImpl {
  meta: StorageBufferSlabAllocatePoolWithHost<DeviceSBTTableMeta>,
  ray_hit: StorageBufferRangeAllocatePool<DeviceHitGroupShaderRecord>,
  hit_offset_map: FastHashMap<u32, u32>,
  ray_miss: StorageBufferRangeAllocatePool<u32>,
  miss_offset_map: FastHashMap<u32, u32>,
  ray_gen: StorageBufferRangeAllocatePool<u32>,
  gen_offset_map: FastHashMap<u32, u32>,
}

// just random number
const SCENE_MESH_INIT_SIZE: u32 = 512;
const SCENE_RAY_TYPE_INIT_SIZE: u32 = 4;
const SCENE_MAX_GROW_RATIO: u32 = 128;

impl ShaderBindingTableDeviceInfo {
  pub fn new(gpu: &GPU) -> Self {
    let inner = ShaderBindingTableDeviceInfoImpl {
      meta: create_storage_buffer_slab_allocate_pool_with_host(gpu, 32, 32 * SCENE_MAX_GROW_RATIO),
      ray_hit: create_storage_buffer_range_allocate_pool(
        gpu,
        SCENE_MESH_INIT_SIZE * SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_MESH_INIT_SIZE * SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
      ray_miss: create_storage_buffer_range_allocate_pool(
        gpu,
        SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
      ray_gen: create_storage_buffer_range_allocate_pool(
        gpu,
        SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
      hit_offset_map: Default::default(),
      miss_offset_map: Default::default(),
      gen_offset_map: Default::default(),
    };
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  pub fn allocate(
    &self,
    max_geometry_count_in_blas: u32,
    max_tlas_offset: u32,
    ray_type_count: u32,
  ) -> Option<u32> {
    let mut inner = self.inner.write();
    let inner: &mut ShaderBindingTableDeviceInfoImpl = &mut inner;
    let hit_group_start = inner.ray_hit.allocate_range(
      max_geometry_count_in_blas * max_tlas_offset * ray_type_count,
      &mut |r| unsafe {
        let meta = inner.hit_offset_map.remove(&r.previous_offset).unwrap();
        inner.hit_offset_map.insert(r.new_offset, meta).unwrap();
        inner
          .meta
          .set_value_sub_bytes(
            meta,
            std::mem::offset_of!(DeviceSBTTableMeta, hit_group_start),
            bytes_of(&r.new_offset),
          )
          .unwrap();
      },
    )?;
    let miss_start = inner
      .ray_miss
      .allocate_range(ray_type_count, &mut |r| unsafe {
        let meta = inner.miss_offset_map.remove(&r.previous_offset).unwrap();
        inner.miss_offset_map.insert(r.new_offset, meta).unwrap();
        inner
          .meta
          .set_value_sub_bytes(
            meta,
            std::mem::offset_of!(DeviceSBTTableMeta, miss_start),
            bytes_of(&r.new_offset),
          )
          .unwrap();
      })?;
    let gen_start = inner
      .ray_gen
      .allocate_range(ray_type_count, &mut |r| unsafe {
        let meta = inner.gen_offset_map.remove(&r.previous_offset).unwrap();
        inner.gen_offset_map.insert(r.new_offset, meta).unwrap();
        inner
          .meta
          .set_value_sub_bytes(
            meta,
            std::mem::offset_of!(DeviceSBTTableMeta, gen_start),
            bytes_of(&r.new_offset),
          )
          .unwrap();
      })?;

    let meta = DeviceSBTTableMeta {
      hit_group_start,
      miss_start,
      gen_start,
      ..Zeroable::zeroed()
    };
    let idx = inner.meta.allocate_value(meta)?;

    inner.miss_offset_map.insert(idx, meta.miss_start);
    inner.hit_offset_map.insert(idx, meta.hit_group_start);
    inner.gen_offset_map.insert(idx, meta.gen_start);

    Some(idx)
  }

  pub fn deallocate(&self, id: u32) {
    let mut inner = self.inner.write();
    let meta = inner.meta.deallocate_back(id).unwrap();
    inner.ray_gen.deallocate(meta.gen_start);
    inner.ray_hit.deallocate(meta.hit_group_start);
    inner.ray_miss.deallocate(meta.miss_start);

    inner.miss_offset_map.remove(&meta.miss_start);
    inner.hit_offset_map.remove(&meta.hit_group_start);
    inner.gen_offset_map.remove(&meta.gen_start);
  }
}

impl ShaderBindingTableDeviceInfo {
  pub fn build(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> ShaderBindingTableDeviceInfoInvocation {
    let inner = self.inner.read();
    ShaderBindingTableDeviceInfoInvocation {
      meta: cx.bind_by(&inner.meta.gpu()),
      ray_hit: cx.bind_by(&inner.ray_hit.gpu()),
      ray_miss: cx.bind_by(&inner.ray_miss.gpu()),
      ray_gen: cx.bind_by(&inner.ray_gen.gpu()),
    }
  }
  pub fn bind(&self, cx: &mut BindingBuilder) {
    let inner = self.inner.read();
    cx.bind(inner.meta.gpu());
    cx.bind(inner.ray_hit.gpu());
    cx.bind(inner.ray_miss.gpu());
    cx.bind(inner.ray_gen.gpu());
  }
}

pub struct ShaderBindingTableDeviceInfoInvocation {
  meta: ShaderAccessorOf<[DeviceSBTTableMeta]>,
  ray_hit: ShaderAccessorOf<[DeviceHitGroupShaderRecord]>,
  ray_miss: ShaderAccessorOf<[u32]>,
  ray_gen: ShaderAccessorOf<[u32]>,
}

impl ShaderBindingTableDeviceInfoInvocation {
  fn get_hit_group(
    &self,
    sbt_id: Node<u32>,
    hit_idx: Node<u32>,
  ) -> ShaderAccessorOf<DeviceHitGroupShaderRecord> {
    let meta = self.meta.index(sbt_id);
    let hit_start = meta.hit_group_start().load();
    self.ray_hit.index(hit_idx + hit_start)
  }

  pub fn get_closest_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    self.get_hit_group(sbt_id, hit_idx).closest_hit().load()
  }

  pub fn get_any_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    self.get_hit_group(sbt_id, hit_idx).any_hit().load()
  }

  pub fn get_intersection_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    self.get_hit_group(sbt_id, hit_idx).intersection().load()
  }

  pub fn get_missing_handle(&self, sbt_id: Node<u32>, idx: Node<u32>) -> Node<u32> {
    let meta = self.meta.index(sbt_id);
    let miss_start = meta.miss_start();
    self.ray_miss.index(miss_start.load() + idx).load()
  }
  pub fn get_ray_gen_handle(&self, sbt_id: Node<u32>) -> Node<u32> {
    let meta = self.meta.index(sbt_id);
    let ray_gen_start = meta.gen_start();
    self.ray_gen.index(ray_gen_start.load()).load()
  }
}

pub type StorageBufferSlabAllocatePoolWithHost<T> =
  SlabAllocatePoolWithHost<StorageBufferReadOnlyDataView<[T]>>;
pub type SlabAllocatePoolWithHost<T> =
  GPUSlatAllocateMaintainer<GrowableHostedDirectQueueUpdateBuffer<T>>;

pub fn create_storage_buffer_slab_allocate_pool_with_host<T: Std430>(
  gpu: &GPU,
  init_size: u32,
  max_size: u32,
) -> StorageBufferSlabAllocatePoolWithHost<T> {
  let buffer = StorageBufferReadOnlyDataView::<[T]>::create_by(
    &gpu.device,
    StorageBufferInit::Zeroed(std::num::NonZeroU64::new(init_size as u64).unwrap()),
  );

  let buffer = create_growable_buffer_with_host_back(gpu, buffer, max_size, true);
  GPUSlatAllocateMaintainer::new(buffer)
}
