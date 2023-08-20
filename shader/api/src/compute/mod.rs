use crate::*;

thread_local! {
  static IN_BUILDING_COMPUTE_SHADER_API: RefCell<Option<DynamicShaderAPI>> = RefCell::new(None);
}

pub struct ShaderComputePipelineBuilder {
  bindgroups: ShaderBindGroupBuilder,
}

impl std::ops::Deref for ShaderComputePipelineBuilder {
  type Target = ShaderBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

pub trait IntoWorkgroupSize {
  fn into_size(self) -> (u32, u32, u32);
}

impl ShaderComputePipelineBuilder {
  pub fn set_work_group_size(&self, size: impl IntoWorkgroupSize) {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()))
  }

  pub fn storage_barrier(&self) {
    call_shader_api(|api| api.barrier(BarrierScope::Storage))
  }

  pub fn workgroup_barrier(&self) {
    call_shader_api(|api| api.barrier(BarrierScope::WorkGroup))
  }
}
