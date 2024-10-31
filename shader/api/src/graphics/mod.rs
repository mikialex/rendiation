use crate::*;

mod vertex;
pub use vertex::*;
mod fragment;
pub use fragment::*;
mod semantic;
pub use semantic::*;

#[derive(Copy, Clone)]
pub enum ShaderVaryingInterpolation {
  Flat,
  Perspective,
}

#[derive(Debug)]
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

  /// todo use upstream any map
  pub context: FastHashMap<TypeId, Box<dyn Any>>,
}

impl ShaderRenderPipelineBuilder {
  fn new(api: &dyn Fn(ShaderStages) -> DynamicShaderAPI) -> Self {
    set_build_api_by(api);
    Self {
      bindgroups: Default::default(),
      vertex: ShaderVertexBuilder::new(),
      fragment: ShaderFragmentBuilder::new(),
      log_result: false,
      context: Default::default(),
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
    logic: impl FnOnce(
      &mut ShaderVertexBuilder,
      &mut ShaderBindGroupBuilder,
    ) -> Result<T, ShaderBuildError>,
  ) -> Result<T, ShaderBuildError> {
    set_current_building(ShaderStages::Vertex.into());
    let result = logic(&mut self.vertex, &mut self.bindgroups)?;
    set_current_building(None);
    Ok(result)
  }
  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(
      &mut ShaderFragmentBuilderView,
      &mut ShaderBindGroupBuilder,
    ) -> Result<T, ShaderBuildError>,
  ) -> Result<T, ShaderBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);
    set_current_building(ShaderStages::Fragment.into());
    let mut builder = ShaderFragmentBuilderView {
      base: &mut self.fragment,
      vertex: &mut self.vertex,
    };
    let result = logic(&mut builder, &mut self.bindgroups)?;
    set_current_building(None);
    Ok(result)
  }

  pub fn build(mut self) -> Result<GraphicsShaderCompileResult, ShaderBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);

    set_current_building(ShaderStages::Vertex.into());
    self.vertex.finalize_position_write();
    set_current_building(None);

    set_current_building(ShaderStages::Fragment.into());
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
        .cloned()
        .map(|(_, s)| s)
        .collect(),
      depth_stencil: self.fragment.depth_stencil,
      multisample: self.fragment.multisample,
    })
  }
}

/// weaker version of GraphicsShaderProvider, only inject shader dependencies
pub trait GraphicsShaderDependencyProvider {
  fn inject_shader_dependencies(&self, builder: &mut ShaderRenderPipelineBuilder);
}

/// The reason why we use two function is that the build process
/// require to generate two separate root scope: two entry main function;
pub trait GraphicsShaderProvider {
  fn build(&self, _builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    // default do nothing
    Ok(())
  }

  fn post_build(&self, _builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    // default do nothing
    Ok(())
  }

  fn build_self(
    &self,
    api_builder: &dyn Fn(ShaderStages) -> DynamicShaderAPI,
  ) -> Result<ShaderRenderPipelineBuilder, ShaderBuildError> {
    let mut builder = ShaderRenderPipelineBuilder::new(api_builder);
    self.build(&mut builder)?;
    self.post_build(&mut builder)?;
    Ok(builder)
  }
}

impl GraphicsShaderProvider for () {}

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
