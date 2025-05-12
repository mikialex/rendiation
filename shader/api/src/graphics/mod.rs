use crate::*;

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

#[derive(Debug, Clone)]
pub enum ShaderBuildError {
  MissingRequiredDependency(&'static str),
  FragmentOutputSlotNotDeclared,
  FailedDowncastShaderValueFromInput,
  SemanticNotSupported,
}

pub struct ShaderRenderPipelineBuilder {
  bindgroups: ShaderBindGroupBuilder,

  pub(crate) vertex: ShaderVertexBuilder,
  pub(crate) fragment: ShaderFragmentBuilder,

  /// Log the shader build result when building shader, for debug purpose.
  pub log_result: bool,

  errors: ErrorSink,
  pub debugger: ShaderBuilderDebugger,
}

impl ShaderRenderPipelineBuilder {
  fn new(api: &dyn Fn(ShaderStage) -> DynamicShaderAPI) -> Self {
    set_build_api_by(api);
    let errors = ErrorSink::new(true);
    Self {
      bindgroups: Default::default(),
      vertex: ShaderVertexBuilder::new(errors.clone()),
      fragment: ShaderFragmentBuilder::new(errors.clone()),
      log_result: false,
      debugger: Default::default(),
      errors,
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

impl ShaderRenderPipelineBuilder {
  pub fn vertex<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderVertexBuilder, &mut ShaderBindGroupBuilder) -> T,
  ) -> T {
    set_current_building(ShaderStage::Vertex.into());
    let result = logic(&mut self.vertex, &mut self.bindgroups);
    set_current_building(None);
    result
  }

  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(&mut ShaderFragmentBuilderView, &mut ShaderBindGroupBuilder) -> T,
  ) -> T {
    self.vertex.sync_fragment_out(&mut self.fragment);
    set_current_building(ShaderStage::Fragment.into());
    let mut builder = ShaderFragmentBuilderView {
      base: &mut self.fragment,
      vertex: &mut self.vertex,
    };
    let result = logic(&mut builder, &mut self.bindgroups);
    set_current_building(None);
    result
  }

  pub fn build(mut self) -> Result<GraphicsShaderCompileResult, ShaderBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);

    set_current_building(ShaderStage::Vertex.into());
    self.vertex.finalize_position_write();
    set_current_building(None);

    set_current_building(ShaderStage::Fragment.into());
    self.fragment.finalize_depth_write();
    set_current_building(None);

    let ShaderBuildingCtx {
      mut vertex,
      mut fragment,
      ..
    } = take_build_api();

    Ok(GraphicsShaderCompileResult {
      vertex_shader: vertex.build(),
      frag_shader: fragment.build(),
      bindings: self.bindgroups,
      vertex_layouts: self.vertex.vertex_layouts,
      primitive_state: self.vertex.primitive_state,
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

/// The reason why we use two function is that the build process
/// require to generate two separate root scope: two entry main function;
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
  ) -> Result<ShaderRenderPipelineBuilder, Vec<ShaderBuildError>> {
    let mut builder = ShaderRenderPipelineBuilder::new(api_builder);
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
    std::any::type_name::<Self>().into()
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
