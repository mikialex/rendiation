use crate::*;

pub mod vertex;
pub use vertex::*;
pub mod fragment;
pub use fragment::*;
pub mod semantic;
pub use semantic::*;
pub mod binding;
pub use binding::*;

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
  // uniforms
  pub bindgroups: ShaderBindGroupBuilder,

  // todo sealed except for codegen
  pub vertex: ShaderVertexBuilder,
  pub fragment: ShaderFragmentBuilder,

  /// Log the shader build result when building shader, for debug purpose.
  pub log_result: bool,

  pub context: FastHashMap<TypeId, Box<dyn Any>>,
}

impl ShaderRenderPipelineBuilder {
  fn new(vertex: DynamicShaderAPI, frag: DynamicShaderAPI) -> Self {
    set_build_api(vertex, frag);
    Self {
      bindgroups: Default::default(),
      vertex: ShaderVertexBuilder::new(),
      fragment: ShaderFragmentBuilder::new(),
      log_result: true,
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
      &mut ShaderBindGroupDirectBuilder,
    ) -> Result<T, ShaderBuildError>,
  ) -> Result<T, ShaderBuildError> {
    set_current_building(ShaderStages::Vertex.into());
    let result = logic(&mut self.vertex, &mut self.bindgroups.wrap())?;
    set_current_building(None);
    Ok(result)
  }
  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(
      &mut ShaderFragmentBuilderView,
      &mut ShaderBindGroupDirectBuilder,
    ) -> Result<T, ShaderBuildError>,
  ) -> Result<T, ShaderBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);
    set_current_building(ShaderStages::Fragment.into());
    let mut builder = ShaderFragmentBuilderView {
      base: &mut self.fragment,
      vertex: &mut self.vertex,
    };
    let result = logic(&mut builder, &mut self.bindgroups.wrap())?;
    set_current_building(None);
    Ok(result)
  }

  pub fn build(mut self) -> Result<ShaderCompileResult, ShaderBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);

    set_current_building(ShaderStages::Vertex.into());
    self.vertex.finalize_position_write();
    set_current_building(None);

    let PipelineShaderAPIPair {
      mut vertex,
      mut fragment,
      ..
    } = take_build_api();

    Ok(ShaderCompileResult {
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
    vertex: DynamicShaderAPI,
    frag: DynamicShaderAPI,
  ) -> Result<ShaderRenderPipelineBuilder, ShaderBuildError> {
    let mut builder = ShaderRenderPipelineBuilder::new(vertex, frag);
    self.build(&mut builder)?;
    self.post_build(&mut builder)?;
    Ok(builder)
  }
}

impl GraphicsShaderProvider for () {}

pub struct ShaderCompileResult {
  pub vertex_shader: (String, Box<dyn Any>),
  pub frag_shader: (String, Box<dyn Any>),
  pub bindings: ShaderBindGroupBuilder,
  pub vertex_layouts: Vec<ShaderVertexBufferLayout>,
  pub primitive_state: PrimitiveState,
  pub color_states: Vec<ColorTargetState>,
  pub depth_stencil: Option<DepthStencilState>,
  pub multisample: MultisampleState,
}

pub(crate) struct PipelineShaderAPIPair {
  vertex: DynamicShaderAPI,
  fragment: DynamicShaderAPI,
  current: Option<ShaderStages>,
}

thread_local! {
  static IN_BUILDING_SHADER_API: RefCell<Option<PipelineShaderAPIPair>> = RefCell::new(None);
}

pub(crate) fn call_shader_api<T>(
  modifier: impl FnOnce(&mut dyn ShaderAPI<Output = Box<dyn Any>>) -> T,
) -> T {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    let api = api.as_mut().unwrap();
    let api = match api.current.unwrap() {
      ShaderStages::Vertex => &mut api.vertex,
      ShaderStages::Fragment => &mut api.fragment,
    }
    .as_mut();

    modifier(api)
  })
}

pub(crate) fn set_current_building(current: Option<ShaderStages>) {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    let api = api.as_mut().unwrap();
    api.current = current
  })
}

pub(crate) fn get_current_stage() -> Option<ShaderStages> {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.as_mut().unwrap().current)
}

pub(crate) fn set_build_api(vertex: DynamicShaderAPI, fragment: DynamicShaderAPI) {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| {
    api.replace(PipelineShaderAPIPair {
      vertex,
      fragment,
      current: None,
    });
  })
}

pub(crate) fn take_build_api() -> PipelineShaderAPIPair {
  IN_BUILDING_SHADER_API.with_borrow_mut(|api| api.take().unwrap())
}
