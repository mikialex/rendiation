use crate::*;

thread_local! {
  static IN_BUILDING_COMPUTE_SHADER_API: RefCell<Option<DynamicShaderAPI>> = RefCell::new(None);
}

pub struct ShaderComputePipelineBuilder {
  pub bindgroups: ShaderBindGroupBuilder,
  pub registry: SemanticRegistry,
  global_invocation_id: Node<Vec3<u32>>,
  local_invocation_id: Node<Vec3<u32>>,
  local_invocation_index: Node<u32>,
  workgroup_id: Node<Vec3<u32>>,
  workgroup_count: Node<Vec3<u32>>,
  // not init these by default because not available on some platform
  // todo, consider late init other struct as well as graphics part(and unify cache mechanism)
  subgroup_id: RwLock<Option<Node<u32>>>,
  subgroup_invocation_id: RwLock<Option<Node<u32>>>,
  subgroup_size: RwLock<Option<Node<u32>>>,
  pub log_result: bool,
  pub checks: ShaderRuntimeChecks,
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

pub fn texture_barrier() {
  // call_shader_api(|api| api.barrier(BarrierScope::Storage))
  println!("warning: texture_barrier is not implemented yet, such call will be ignored");
}

pub fn workgroup_barrier() {
  call_shader_api(|api| api.barrier(BarrierScope::WorkGroup))
}

/// Calling this function requires SUBGROUP_BARRIER feature.
pub fn subgroup_barrier() {
  call_shader_api(|api| api.barrier(BarrierScope::SubGroup))
}

/// Returns the value pointed to by p to all invocations in the workgroup.
/// The return value is uniform. p must be a uniform value.
///
/// User must ensure the underlayer memory space is workgroup.
pub fn workgroup_uniform_load<T: ShaderSizedValueNodeType>(p: ShaderPtrOf<T>) -> Node<T> {
  call_shader_api(|api| unsafe {
    let pointer = p.raw().get_raw_ptr();
    api
      .make_expression(ShaderNodeExpr::WorkGroupUniformLoad {
        pointer,
        ty: T::sized_ty(),
      })
      .into_node()
  })
}

impl ShaderComputePipelineBuilder {
  pub fn new(api: &dyn Fn(ShaderStage) -> DynamicShaderAPI, checks: ShaderRuntimeChecks) -> Self {
    set_build_api_by(api);

    set_current_building(ShaderStage::Compute.into());

    use ShaderBuiltInDecorator::*;
    let r = Self {
      checks,
      bindgroups: Default::default(),
      registry: Default::default(),
      global_invocation_id: ShaderInputNode::BuiltIn(CompGlobalInvocationId).insert_api(),
      local_invocation_id: ShaderInputNode::BuiltIn(CompLocalInvocationId).insert_api(),
      local_invocation_index: ShaderInputNode::BuiltIn(CompLocalInvocationIndex).insert_api(),
      workgroup_id: ShaderInputNode::BuiltIn(CompWorkgroupId).insert_api(),
      workgroup_count: ShaderInputNode::BuiltIn(CompNumWorkgroup).insert_api(),
      subgroup_id: Default::default(),
      subgroup_invocation_id: Default::default(),
      subgroup_size: Default::default(),
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

  pub fn subgroup_invocation_id(&self) -> Node<u32> {
    *self.subgroup_invocation_id.write().get_or_insert_with(|| {
      ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::CompSubgroupInvocationId).insert_api()
    })
  }
  pub fn subgroup_id(&self) -> Node<u32> {
    *self.subgroup_id.write().get_or_insert_with(|| {
      ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::CompSubgroupId).insert_api()
    })
  }
  pub fn subgroup_size(&self) -> Node<u32> {
    *self.subgroup_size.write().get_or_insert_with(|| {
      ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::CompSubgroupSize).insert_api()
    })
  }

  pub fn define_workgroup_shared_var<T: ShaderSizedValueNodeType>(&self) -> ShaderPtrOf<T> {
    let handle = ShaderInputNode::WorkGroupShared { ty: T::sized_ty() }.insert_api_raw();
    T::create_view_from_raw_ptr(Box::new(handle))
  }
  pub fn define_workgroup_shared_var_host_size_array<T: ShaderSizedValueNodeType>(
    &self,
    len: u32,
  ) -> ShaderPtrOf<HostDynSizeArray<T>> {
    let ty = ShaderSizedValueType::FixedSizeArray(Box::new(T::sized_ty()), len as usize);
    let handle = ShaderInputNode::WorkGroupShared { ty }.insert_api_raw();
    StaticLengthArrayView {
      phantom: PhantomData,
      array: PhantomData,
      access: Box::new(handle),
      len,
    }
  }
  pub fn define_invocation_private_var<T: ShaderSizedValueNodeType>(&self) -> ShaderPtrOf<T> {
    let handle = ShaderInputNode::Private { ty: T::sized_ty() }.insert_api_raw();
    T::create_view_from_raw_ptr(Box::new(handle))
  }

  pub fn bindgroups(&mut self) -> &mut ShaderBindGroupBuilder {
    &mut self.bindgroups
  }

  pub fn bind_by<T: AbstractShaderBindingSource>(&mut self, instance: &T) -> T::ShaderBindResult {
    self.bindgroups().bind_by(instance)
  }

  pub fn bind_single_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    self.bindgroups().bind_single_by(instance)
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
