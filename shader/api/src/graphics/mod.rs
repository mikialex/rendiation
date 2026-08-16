use crate::*;

mod task_mesh;
pub use task_mesh::*;
mod vertex;
pub use vertex::*;
mod fragment;
pub use fragment::*;
mod semantic;
pub use semantic::*;
mod debugger;
pub use debugger::*;
mod error_sink;
pub(crate) use error_sink::*;
mod high_precision_translation;
pub use high_precision_translation::*;

#[derive(Debug, Clone)]
pub enum ShaderBuildError {
  MissingRequiredDependency(&'static str, Location<'static>),
  FragmentOutputSlotNotDeclared,
  FailedDowncastShaderValueFromInput,
  SemanticNotSupported,
}

pub struct ShaderRenderPipelineBuilder {
  bindgroups: ShaderBindGroupBuilder,
  pub checks: ShaderRuntimeChecks,

  pub(crate) shape: Box<dyn AbstractShaderVertexBuilder>,
  pub(crate) fragment: ShaderFragmentBuilder,

  errors: ErrorSink,
  pub debugger: ShaderBuilderDebugger,
  pub info: Arc<GPUInfo>,
}

#[derive(Debug, Clone)]
pub struct GPUInfo {
  pub adaptor_info: wgpu_types::AdapterInfo,
  pub power_preference: wgpu_types::PowerPreference,
  pub supported_features: wgpu_types::Features,
  pub supported_limits: wgpu_types::Limits,
  pub downgrade_info: wgpu_types::DownlevelCapabilities,
}

impl GPUInfo {
  pub fn is_webgl(&self) -> bool {
    #[cfg(target_family = "wasm")]
    {
      self.adaptor_info.backend == wgpu_types::Backend::Gl
    }
    #[cfg(not(target_family = "wasm"))]
    {
      false
    }
  }
}

impl ShaderRenderPipelineBuilder {
  fn new(
    use_mesh_shader: bool,
    api: &dyn Fn(ShaderStage) -> DynamicShaderAPI,
    info: Arc<GPUInfo>,
    checks: ShaderRuntimeChecks,
  ) -> Self {
    set_build_api_by(api);
    let errors = ErrorSink::new(true);
    Self {
      bindgroups: Default::default(),
      shape: if use_mesh_shader {
        unimplemented!()
      } else {
        Box::new(ShaderRawVertexBuilder::new(errors.clone()))
      },
      fragment: ShaderFragmentBuilder::new(errors.clone()),
      debugger: Default::default(),
      errors,
      info,
      checks,
    }
  }
}

impl std::ops::Deref for ShaderRenderPipelineBuilder {
  type Target = ShaderBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderRenderPipelineBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

pub trait AbstractShaderVertexBuilder {
  fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder>;
  fn expect_vertex_shader(&mut self) -> &mut ShaderRawVertexBuilder {
    self.vertex_shader().expect(
      "unable to get vertex-shader-only ability (such as vertex buffer) in mesh-task shader stages",
    )
  }
  fn set_current_building(&mut self);
  fn finalize_write(&mut self);
  fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder);
  fn register_impl(&mut self, ty_id: TypeId, node: NodeUntyped);
  fn try_query_impl(&mut self, ty_id: TypeId) -> Option<NodeUntyped>;
  fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  );
  fn primitive_state(&mut self) -> &mut PrimitiveState;
  fn registry(&mut self) -> &mut SemanticRegistry;
  fn error(&mut self, err: ShaderBuildError);
}

pub trait AbstractShaderVertexBuilderTypedExt {
  fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>);
  fn try_query<T: SemanticVertexShaderValue>(&mut self) -> Option<Node<T::ValueType>>;
  fn query<T: SemanticVertexShaderValue>(&mut self) -> Node<T::ValueType>;
  fn query_or_insert_by<T>(&mut self, by: impl FnOnce() -> T::ValueType) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType;
  fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType;
  fn set_vertex_out<T>(&mut self, node: impl Into<Node<T::ValueType>>)
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType;
  fn set_vertex_out_with_given_interpolate<T>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
    interpolation: ShaderInterpolation,
  ) where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType;
  fn register_any<T: Any>(&mut self, value: T);
}

impl<B: AbstractShaderVertexBuilder + ?Sized> AbstractShaderVertexBuilderTypedExt for B {
  fn register<T: SemanticVertexShaderValue>(&mut self, node: impl Into<Node<T::ValueType>>) {
    let node = node.into().cast_untyped_node();
    node.mark_debug_label(get_name::<T>());
    self.register_impl(TypeId::of::<T>(), node);
  }

  fn try_query<T: SemanticVertexShaderValue>(&mut self) -> Option<Node<T::ValueType>> {
    self
      .try_query_impl(TypeId::of::<T>())
      .map(|n| unsafe { n.cast_type() })
  }

  #[track_caller]
  fn query<T: SemanticVertexShaderValue>(&mut self) -> Node<T::ValueType> {
    let location = *Location::caller();
    self.try_query::<T>().unwrap_or_else(|| unsafe {
      self.error(ShaderBuildError::MissingRequiredDependency(
        T::NAME,
        location,
      ));
      fake_val()
    })
  }

  fn query_or_insert_by<T>(&mut self, by: impl FnOnce() -> T::ValueType) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    if let Some(n) = self.try_query::<T>() {
      n
    } else {
      let default: T::ValueType = by();
      self.register::<T>(default);
      self.query::<T>()
    }
  }

  fn query_or_insert_default<T>(&mut self) -> Node<T::ValueType>
  where
    T: SemanticVertexShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.query_or_insert_by::<T>(Default::default)
  }

  fn set_vertex_out_with_given_interpolate<T>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
    interpolation: ShaderInterpolation,
  ) where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    let node = node.into().cast_untyped_node();
    self.set_vertex_out_impl(
      TypeId::of::<T>(),
      T::ValueType::primitive_ty(),
      node,
      interpolation,
    );
  }

  fn set_vertex_out<T>(&mut self, node: impl Into<Node<T::ValueType>>)
  where
    T: SemanticFragmentShaderValue,
    T::ValueType: PrimitiveShaderNodeType,
  {
    self.set_vertex_out_with_given_interpolate::<T>(node, ShaderInterpolation::Perspective)
  }

  fn register_any<T: Any>(&mut self, value: T) {
    self.registry().any_map.register(value);
  }
}

impl ShaderRenderPipelineBuilder {
  pub fn vertex<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderVertexBuilder, &mut ShaderBindGroupBuilder) -> T,
  ) -> T {
    self.shape.set_current_building();
    let mut builder = ShaderVertexBuilder {
      internal: self.shape.as_mut(),
    };
    let result = logic(&mut builder, &mut self.bindgroups);
    set_current_building(None);
    result
  }

  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderFragmentBuilderView, &mut ShaderBindGroupBuilder) -> T,
  ) -> T {
    self.shape.sync_fragment_out(&mut self.fragment);
    set_current_building(ShaderStage::Fragment.into());
    let mut builder = ShaderFragmentBuilderView {
      base: &mut self.fragment,
      shape: self.shape.as_mut(),
    };
    let result = logic(&mut builder, &mut self.bindgroups);
    set_current_building(None);
    result
  }

  pub fn build(mut self) -> Result<GraphicsShaderCompileResult, ShaderBuildError> {
    self.shape.sync_fragment_out(&mut self.fragment);

    self.shape.set_current_building();
    self.shape.finalize_write();
    set_current_building(None);

    set_current_building(ShaderStage::Fragment.into());
    self.fragment.finalize_depth_write();
    set_current_building(None);

    let vertex_layouts = if let Some(raw_vertex) = self.shape.vertex_shader() {
      raw_vertex.vertex_layouts.clone()
    } else {
      Vec::new()
    };

    let ShaderBuildingCtx {
      mut vertex,
      mut fragment,
      ..
    } = take_build_api();

    Ok(GraphicsShaderCompileResult {
      vertex_shader: vertex.build(),
      frag_shader: fragment.build(),
      bindings: self.bindgroups,
      vertex_layouts,
      primitive_state: *self.shape.primitive_state(),
      color_states: self
        .fragment
        .frag_output
        .iter()
        .map(|p| &p.states)
        .cloned()
        .collect(),
      depth_stencil: self.fragment.depth_stencil,
      multisample: self.fragment.multisample,
    })
  }
}

pub trait GraphicsShaderProvider {
  fn build(&self, _builder: &mut ShaderRenderPipelineBuilder) {
    // do nothing in default
  }

  fn post_build(&self, _builder: &mut ShaderRenderPipelineBuilder) {
    // do nothing in default
  }

  fn build_self(
    &self,
    api_builder: &dyn Fn(ShaderStage) -> DynamicShaderAPI,
    info: Arc<GPUInfo>,
    checks: ShaderRuntimeChecks,
    use_mesh_shader: bool,
  ) -> Result<ShaderRenderPipelineBuilder, Vec<ShaderBuildError>> {
    let mut builder = ShaderRenderPipelineBuilder::new(use_mesh_shader, api_builder, info, checks);
    self.build(&mut builder);
    self.post_build(&mut builder);
    let errors = builder.errors.finish();
    if errors.is_empty() {
      Ok(builder)
    } else {
      Err(errors)
    }
  }

  fn debug_label(&self) -> String {
    disqualified::ShortName::of::<Self>().to_string()
  }
}

impl GraphicsShaderProvider for () {}
impl<T: GraphicsShaderProvider> GraphicsShaderProvider for &T {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (*self).build(builder);
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (*self).post_build(builder);
  }
}

pub struct GraphicsShaderCompileResult {
  pub vertex_shader: (String, Box<dyn Any>),
  pub frag_shader: (String, Box<dyn Any>),
  pub bindings: ShaderBindGroupBuilder,
  pub vertex_layouts: Vec<ShaderVertexBufferLayout>,
  pub primitive_state: PrimitiveState,
  pub color_states: Vec<ColorTargetState>,
  pub depth_stencil: Option<DepthStencilState>,
  pub multisample: MultisampleState,
}
