use crate::*;

thread_local! {
  static IN_BUILDING_COMPUTE_SHADER_API: RefCell<Option<DynamicShaderAPI>> = RefCell::new(None);
}

pub struct ShaderComputePipelineBuilder {
  bindgroups: ShaderBindGroupBuilder,
  global_invocation_id: Node<Vec3<u32>>,
  local_invocation_id: Node<Vec3<u32>>,
  local_invocation_index: Node<u32>,
  workgroup_id: Node<Vec3<u32>>,
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
  pub fn new(api: &dyn Fn(ShaderStages) -> DynamicShaderAPI) -> Self {
    set_build_api(api);

    set_current_building(ShaderStages::Compute.into());

    use ShaderBuiltInDecorator::*;
    Self {
      bindgroups: Default::default(),
      global_invocation_id: ShaderInputNode::BuiltIn(CompGlobalInvocationId).insert_api(),
      local_invocation_id: ShaderInputNode::BuiltIn(CompLocalInvocationId).insert_api(),
      local_invocation_index: ShaderInputNode::BuiltIn(CompLocalInvocationIndex).insert_api(),
      workgroup_id: ShaderInputNode::BuiltIn(CompWorkgroupId).insert_api(),
    }
  }

  pub fn set_work_group_size(&self, size: impl IntoWorkgroupSize) {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()))
  }

  pub fn storage_barrier(&self) {
    call_shader_api(|api| api.barrier(BarrierScope::Storage))
  }

  pub fn workgroup_barrier(&self) {
    call_shader_api(|api| api.barrier(BarrierScope::WorkGroup))
  }

  pub fn global_invocation_id(&self) -> Node<Vec3<u32>> {
    self.global_invocation_id
  }

  pub fn local_invocation_id(&self) -> Node<Vec3<u32>> {
    self.local_invocation_id
  }

  /// https://www.w3.org/TR/WGSL/#local-invocation-index
  pub fn local_invocation_index(&self) -> Node<u32> {
    self.local_invocation_index
  }

  pub fn workgroup_id(&self) -> Node<Vec3<u32>> {
    self.workgroup_id
  }

  pub fn build(self) -> Result<ComputeShaderCompileResult, ShaderBuildError> {
    let ShaderBuildingCtx { mut compute, .. } = take_build_api();

    Ok(ComputeShaderCompileResult {
      shader: compute.build(),
      bindings: self.bindgroups,
    })
  }
}

pub struct ComputeShaderCompileResult {
  pub shader: (String, Box<dyn Any>),
  pub bindings: ShaderBindGroupBuilder,
}
