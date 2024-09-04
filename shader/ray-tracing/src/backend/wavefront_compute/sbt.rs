use crate::*;

pub struct ShaderBindingTableInfo {
  pub ray_generation: Option<ShaderHandle>,
  pub ray_miss: Vec<Option<ShaderHandle>>, // ray_type_count size
  pub ray_hit: Vec<HitGroupShaderRecord>,  // mesh_count size
  pub(crate) sys: ShaderBindingTableDeviceInfo,
  pub(crate) self_idx: u32,
}

// todo support resize
impl ShaderBindingTableProvider for ShaderBindingTableInfo {
  fn config_ray_generation(&mut self, s: ShaderHandle) {
    let sys = self.sys.inner.read();
    // let ray_gen_start = sys.meta
    // sys.ray_gen
    todo!()
  }

  fn config_hit_group(&mut self, mesh_idx: u32, hit_group: HitGroupShaderRecord) {
    todo!()
  }

  fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle) {
    todo!()
  }
  fn access_impl(&self) -> &dyn Any {
    self
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DeviceSBTTableMeta {
  pub hit_group_start: u32,
  pub miss_start: u32,
  pub gen_start: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DeviceHistGroupShaderRecord {
  pub closet_hit: u32,
  pub any_hit: u32,
  pub intersection: u32,
}

#[derive(Clone)]
pub struct ShaderBindingTableDeviceInfo {
  gpu: GPU,
  inner: Arc<RwLock<ShaderBindingTableDeviceInfoImpl>>,
}

pub struct ShaderBindingTableDeviceInfoImpl {
  meta: VecWithStorageBuffer<DeviceSBTTableMeta>,
  ray_hit: StorageBufferRangeAllocatePool<DeviceHistGroupShaderRecord>,
  ray_miss: StorageBufferRangeAllocatePool<u32>,
  ray_gen: StorageBufferRangeAllocatePool<u32>,
}

// just random number
const SCENE_MESH_INIT_SIZE: usize = 512;
const SCENE_RAY_TYPE_INIT_SIZE: usize = 4;
const SCENE_MAX_GROW_RATIO: usize = 128;

impl ShaderBindingTableDeviceInfo {
  pub fn new(gpu: &GPU) -> Self {
    let inner = ShaderBindingTableDeviceInfoImpl {
      meta: VecWithStorageBuffer::new(&gpu.device, 32, 32 * SCENE_MAX_GROW_RATIO),
      ray_hit: StorageBufferRangeAllocatePool::new(
        &gpu.device,
        SCENE_MESH_INIT_SIZE * SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_MESH_INIT_SIZE * SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
      ray_miss: StorageBufferRangeAllocatePool::new(
        &gpu.device,
        SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
      ray_gen: StorageBufferRangeAllocatePool::new(
        &gpu.device,
        SCENE_RAY_TYPE_INIT_SIZE,
        SCENE_RAY_TYPE_INIT_SIZE * SCENE_MAX_GROW_RATIO,
      ),
    };
    Self {
      gpu: gpu.clone(),
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  pub fn allocate(&self, mesh_count: u32, ray_type_count: u32) -> u32 {
    let mut inner = self.inner.write();
    inner.ray_hit.allocate(
      todo!(),
      todo!(),
      // content: &[u8],
      &self.gpu.device,
      &self.gpu.queue,
      &mut |_| {
        //
      },
    );
    // inner.ray_miss.allocate(
    //   allocation_handle,
    //   content,
    //   device,
    //   queue,
    //   relocation_handler,
    // );
    // inner.ray_gen.allocate(
    //   allocation_handle,
    //   content,
    //   device,
    //   queue,
    //   relocation_handler,
    // );
    todo!()
  }

  pub fn deallocate(&self, id: u32) {
    todo!()
  }
}

impl ShaderBindingTableDeviceInfo {
  pub fn build(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> ShaderBindingTableDeviceInfoInvocation {
    let inner = self.inner.read();
    ShaderBindingTableDeviceInfoInvocation {
      meta: cx.bind_by(&inner.meta),
      ray_hit: cx.bind_by(&inner.ray_hit),
      ray_miss: cx.bind_by(&inner.ray_miss),
      ray_gen: cx.bind_by(&inner.ray_gen),
    }
  }
  pub fn bind(&self, cx: &mut BindingBuilder) {
    let inner = self.inner.read();
    cx.bind(&inner.meta);
    cx.bind(&inner.ray_hit);
    cx.bind(&inner.ray_miss);
    cx.bind(&inner.ray_gen);
  }
}

pub struct ShaderBindingTableDeviceInfoInvocation {
  meta: ReadOnlyStorageNode<[DeviceSBTTableMeta]>,
  ray_hit: ReadOnlyStorageNode<[DeviceHistGroupShaderRecord]>,
  ray_miss: ReadOnlyStorageNode<[u32]>,
  ray_gen: ReadOnlyStorageNode<[u32]>,
}

// todo improve code by pointer struct field access macro
impl ShaderBindingTableDeviceInfoInvocation {
  pub fn get_closest_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    let hit_start = self.meta.index(sbt_id).load().expand().hit_group_start; // todo fix over expand
    let hit_group = self.ray_hit.index(hit_idx + hit_start).handle();
    let handle: StorageNode<u32> = unsafe { index_access_field(hit_group, 0) };
    handle.load()
  }

  pub fn get_any_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    let hit_start = self.meta.index(sbt_id).load().expand().hit_group_start; // todo fix over expand
    let hit_group = self.ray_hit.index(hit_idx + hit_start).handle();
    let handle: StorageNode<u32> = unsafe { index_access_field(hit_group, 1) };
    handle.load()
  }

  pub fn get_intersection_handle(&self, sbt_id: Node<u32>, hit_idx: Node<u32>) -> Node<u32> {
    let hit_start = self.meta.index(sbt_id).load().expand().hit_group_start; // todo fix over expand
    let hit_group = self.ray_hit.index(hit_idx + hit_start).handle();
    let handle: StorageNode<u32> = unsafe { index_access_field(hit_group, 1) };
    handle.load()
  }

  pub fn get_missing_handle(&self, sbt_id: Node<u32>, idx: Node<u32>) -> Node<u32> {
    let miss_start = self.meta.index(sbt_id).load().expand().miss_start; // todo fix over expand
    self.ray_miss.index(miss_start + idx).load()
  }
  pub fn get_ray_gen_handle(&self, sbt_id: Node<u32>) -> Node<u32> {
    let ray_gen = self.meta.index(sbt_id).load().expand().gen_start; // todo fix over expand
    self.ray_gen.index(ray_gen).load()
  }
}
