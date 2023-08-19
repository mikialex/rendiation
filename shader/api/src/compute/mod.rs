use crate::*;

thread_local! {
  static IN_BUILDING_COMPUTE_SHADER_API: RefCell<Option<DynamicShaderAPI>> = RefCell::new(None);
}

pub struct ShaderComputePipelineBuilder {
  // bindgroups: ShaderBindGroupBuilder,
}

impl ShaderComputePipelineBuilder {
  pub fn set_work_group_size(&mut self) {
    //
  }
}
