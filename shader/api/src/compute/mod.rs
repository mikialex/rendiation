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
  workgroup_count: Node<Vec3<u32>>,
  pub log_result: bool,
}

pub trait IntoWorkgroupSize {
  fn into_size(self) -> (u32, u32, u32);
}

impl IntoWorkgroupSize for u32 {
  fn into_size(self) -> (u32, u32, u32) {
    (self, 1, 1)
  }
}

impl IntoWorkgroupSize for (u32, u32) {
  fn into_size(self) -> (u32, u32, u32) {
    (self.0, self.1, 1)
  }
}

impl IntoWorkgroupSize for (u32, u32, u32) {
  fn into_size(self) -> (u32, u32, u32) {
    self
  }
}

pub fn storage_barrier() {
  call_shader_api(|api| api.barrier(BarrierScope::Storage))
}

pub fn workgroup_barrier() {
  call_shader_api(|api| api.barrier(BarrierScope::WorkGroup))
}

impl ShaderComputePipelineBuilder {
  pub fn new(api: &dyn Fn(ShaderStage) -> DynamicShaderAPI) -> Self {
    set_build_api_by(api);

    set_current_building(ShaderStage::Compute.into());

    use ShaderBuiltInDecorator::*;
    let r = Self {
      bindgroups: Default::default(),
      global_invocation_id: ShaderInputNode::BuiltIn(CompGlobalInvocationId).insert_api(),
      local_invocation_id: ShaderInputNode::BuiltIn(CompLocalInvocationId).insert_api(),
      local_invocation_index: ShaderInputNode::BuiltIn(CompLocalInvocationIndex).insert_api(),
      workgroup_id: ShaderInputNode::BuiltIn(CompWorkgroupId).insert_api(),
      workgroup_count: ShaderInputNode::BuiltIn(CompNumWorkgroup).insert_api(),
      log_result: false,
    };

    // if user not setting any workgroup size in building process, we use this as default config
    r.with_config_work_group_size(256)
  }

  pub fn with_config_work_group_size(self, size: impl IntoWorkgroupSize) -> Self {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()));
    self
  }

  pub fn config_work_group_size(&self, size: impl IntoWorkgroupSize) -> &Self {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()));
    self
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

  pub fn workgroup_count(&self) -> Node<Vec3<u32>> {
    self.workgroup_count
  }

  pub fn define_workgroup_shared_var<T: ShaderSizedValueNodeType>(&self) -> ShaderAccessorOf<T> {
    let handle = ShaderInputNode::WorkGroupShared { ty: T::sized_ty() }
      .insert_api::<AnyType>()
      .handle();
    T::create_accessor_from_raw_ptr(Box::new(handle))
  }
  pub fn define_workgroup_shared_var_host_size_array<T: ShaderSizedValueNodeType>(
    &self,
    len: u32,
  ) -> ShaderAccessorOf<HostDynSizeArray<T>> {
    let ty = ShaderSizedValueType::FixedSizeArray(Box::new(T::sized_ty()), len as usize);
    let handle = ShaderInputNode::WorkGroupShared { ty }
      .insert_api::<AnyType>()
      .handle();
    HostDynSizeArray::<T>::create_accessor_from_raw_ptr(Box::new(handle))
  }
  pub fn define_invocation_private_var<T: ShaderSizedValueNodeType>(&self) -> ShaderAccessorOf<T> {
    let handle = ShaderInputNode::Private { ty: T::sized_ty() }
      .insert_api::<AnyType>()
      .handle();
    T::create_accessor_from_raw_ptr(Box::new(handle))
  }

  pub fn bindgroups(&mut self) -> &mut ShaderBindGroupBuilder {
    &mut self.bindgroups
  }

  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    self.bindgroups().bind_by(instance)
  }

  pub fn with_log_shader(mut self) -> Self {
    self.log_result = true;
    self
  }
  pub fn enable_log_shader(&mut self) -> &mut Self {
    self.log_result = true;
    self
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
