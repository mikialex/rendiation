use crate::*;

pub struct ShaderBindingTableInfo {
  pub ray_generation: ShaderHandle,
  pub ray_miss: Vec<ShaderHandle>,        // ray_type_count size
  pub ray_hit: Vec<HitGroupShaderRecord>, // mesh_count size
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
  fn access_impl(&mut self) -> &mut dyn Any {
    self
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DeviceHistGroupShaderRecord {
  pub closet_hit: u32,
  pub any_hit: u32,
  pub intersection: u32,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DeviceRayGenAndMissShaderRecord {
  pub ray_gen: u32,
  pub ray_miss: Shader140Array<u32, 8>,
}

pub struct ShaderBindingTableDeviceInfo {
  ray_hit: StorageBufferReadOnlyDataView<[DeviceHistGroupShaderRecord]>,
  ray_miss_and_gen: UniformBufferDataView<DeviceRayGenAndMissShaderRecord>,
}

impl ShaderBindingTableDeviceInfo {
  pub fn build(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> ShaderBindingTableDeviceInfoInvocation {
    ShaderBindingTableDeviceInfoInvocation {
      ray_hit: cx.bind_by(&self.ray_hit),
      ray_miss_and_gen: cx.bind_by(&self.ray_miss_and_gen),
    }
  }
  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.ray_hit);
    cx.bind(&self.ray_miss_and_gen);
  }
}

impl ShaderBindingTableInfo {
  pub fn new(mesh_count: u32, ray_type_count: u32) -> Self {
    ShaderBindingTableInfo {
      ray_generation: todo!(),
      ray_miss: todo!(),
      ray_hit: todo!(),
    }
  }
}

pub struct ShaderBindingTableDeviceInfoInvocation {
  ray_hit: ReadOnlyStorageNode<[DeviceHistGroupShaderRecord]>,
  ray_miss_and_gen: UniformNode<DeviceRayGenAndMissShaderRecord>,
}

// todo improve code by pointer struct field access macro
impl ShaderBindingTableDeviceInfoInvocation {
  pub fn get_closest_handle(&self, hit_idx: Node<u32>) -> Node<u32> {
    let hit_group = self.ray_hit.index(hit_idx).handle();
    let handle: UniformNode<u32> = unsafe { index_access_field(hit_group, 0) };
    handle.load()
  }

  pub fn get_any_handle(&self, hit_idx: Node<u32>) -> Node<u32> {
    let hit_group = self.ray_hit.index(hit_idx).handle();
    let handle: UniformNode<u32> = unsafe { index_access_field(hit_group, 1) };
    handle.load()
  }

  pub fn get_intersection_handle(&self, hit_idx: Node<u32>) -> Node<u32> {
    let hit_group = self.ray_hit.index(hit_idx).handle();
    let handle: UniformNode<u32> = unsafe { index_access_field(hit_group, 1) };
    handle.load()
  }

  pub fn get_missing_handle(&self, idx: Node<u32>) -> Node<u32> {
    let miss_handles: UniformNode<Shader140Array<u32, 8>> =
      unsafe { index_access_field(self.ray_miss_and_gen.handle(), 1) };
    miss_handles.index(idx).load()
  }
  pub fn get_ray_gen_handle(&self) -> Node<u32> {
    let gen_handle: UniformNode<u32> =
      unsafe { index_access_field(self.ray_miss_and_gen.handle(), 0) };
    gen_handle.load()
  }
}
