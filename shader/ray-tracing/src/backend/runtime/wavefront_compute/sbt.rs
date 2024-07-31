use crate::*;

pub struct ShaderBindingTableInfo {
  pub ray_generation: ShaderHandle,
  pub ray_miss: Vec<ShaderHandle>,        // ray_type_count size
  pub ray_hit: Vec<HitGroupShaderRecord>, // mesh_count size
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
  pub gen: u32,
  pub miss: Shader140Array<u32, 8>,
}

pub struct ShaderBindingTableDeviceInfo {
  ray_hit: StorageBufferReadOnlyDataView<[DeviceHistGroupShaderRecord]>,
  ray_miss_and_gen: UniformBufferDataView<DeviceRayGenAndMissShaderRecord>,
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
}
