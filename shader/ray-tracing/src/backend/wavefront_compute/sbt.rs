use crate::*;

pub struct ShaderBindingTableInfo {
  pub ray_generation: Option<ShaderHandle>,
  pub ray_miss: Vec<Option<ShaderHandle>>, // ray_type_count size
  pub ray_hit: Vec<HitGroupShaderRecord>,  // mesh_count size
  pub(crate) sys: ShaderBindingTableDeviceInfo,
  pub(crate) self_idx: u32,
}

impl ShaderBindingTableProvider for ShaderBindingTableInfo {
  fn resize(&mut self, mesh_count: u32, ray_type_count: u32) {
    todo!()
  }

  fn config_ray_generation(&mut self, s: ShaderHandle) {
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
  meta: StorageBufferReadOnlyDataView<[DeviceSBTTableMeta]>,
  ray_hit: StorageBufferReadOnlyDataView<[DeviceHistGroupShaderRecord]>,
  ray_miss: StorageBufferReadOnlyDataView<[u32]>,
  ray_gen: StorageBufferReadOnlyDataView<[u32]>,
}

impl ShaderBindingTableDeviceInfo {
  pub fn new(gpu: &GPU) -> Self {
    todo!()
  }

  pub fn allocate(&self) -> u32 {
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
    ShaderBindingTableDeviceInfoInvocation {
      meta: cx.bind_by(&self.meta),
      ray_hit: cx.bind_by(&self.ray_hit),
      ray_miss: cx.bind_by(&self.ray_miss),
      ray_gen: cx.bind_by(&self.ray_gen),
    }
  }
  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.meta);
    cx.bind(&self.ray_hit);
    cx.bind(&self.ray_miss);
    cx.bind(&self.ray_gen);
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
