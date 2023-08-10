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
pub enum ShaderGraphBuildError {
  MissingRequiredDependency(&'static str),
  FragmentOutputSlotNotDeclared,
  FailedDowncastShaderValueFromInput,
  SemanticNotSupported,
}

pub struct ShaderGraphRenderPipelineBuilder {
  // uniforms
  pub bindgroups: ShaderGraphBindGroupBuilder,

  // todo sealed except for codegen
  pub vertex: ShaderGraphVertexBuilder,
  pub fragment: ShaderGraphFragmentBuilder,

  /// Log the shader build result when building shader, for debug purpose.
  pub log_result: bool,

  pub context: FastHashMap<TypeId, Box<dyn Any>>,
}

impl ShaderGraphRenderPipelineBuilder {
  fn new(vertex: Box<dyn ShaderAPI>, frag: Box<dyn ShaderAPI>) -> Self {
    set_build_graph(vertex, frag);
    Self {
      bindgroups: Default::default(),
      vertex: ShaderGraphVertexBuilder::new(),
      fragment: ShaderGraphFragmentBuilder::new(),
      log_result: false,
      context: Default::default(),
    }
  }
}

impl std::ops::Deref for ShaderGraphRenderPipelineBuilder {
  type Target = ShaderGraphBindGroupBuilder;

  fn deref(&self) -> &Self::Target {
    &self.bindgroups
  }
}

impl std::ops::DerefMut for ShaderGraphRenderPipelineBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bindgroups
  }
}

impl ShaderGraphRenderPipelineBuilder {
  pub fn vertex<T>(
    &mut self,
    logic: impl FnOnce(
      &mut ShaderGraphVertexBuilder,
      &mut ShaderGraphBindGroupDirectBuilder,
    ) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    set_current_building(ShaderStages::Vertex.into());
    let result = logic(&mut self.vertex, &mut self.bindgroups.wrap())?;
    set_current_building(None);
    Ok(result)
  }
  pub fn fragment<T>(
    &mut self,
    logic: impl FnOnce(
      &mut ShaderGraphFragmentBuilderView,
      &mut ShaderGraphBindGroupDirectBuilder,
    ) -> Result<T, ShaderGraphBuildError>,
  ) -> Result<T, ShaderGraphBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);
    set_current_building(ShaderStages::Fragment.into());
    let mut builder = ShaderGraphFragmentBuilderView {
      base: &mut self.fragment,
      vertex: &mut self.vertex,
    };
    let result = logic(&mut builder, &mut self.bindgroups.wrap())?;
    set_current_building(None);
    Ok(result)
  }

  pub fn build(mut self) -> Result<ShaderGraphCompileResult, ShaderGraphBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);

    let PipelineShaderGraphPair {
      mut vertex,
      mut fragment,
      ..
    } = take_build_graph();

    Ok(ShaderGraphCompileResult {
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
  fn build(
    &self,
    _builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }

  fn post_build(
    &self,
    _builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }

  fn build_self(
    &self,
    vertex: Box<dyn ShaderAPI>,
    frag: Box<dyn ShaderAPI>,
  ) -> Result<ShaderGraphRenderPipelineBuilder, ShaderGraphBuildError> {
    let mut builder = ShaderGraphRenderPipelineBuilder::new(vertex, frag);
    self.build(&mut builder)?;
    self.post_build(&mut builder)?;
    Ok(builder)
  }
}

impl GraphicsShaderProvider for () {}

pub struct ShaderGraphCompileResult {
  pub vertex_shader: (String, String),
  pub frag_shader: (String, String),
  pub bindings: ShaderGraphBindGroupBuilder,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,
  pub color_states: Vec<ColorTargetState>,
  pub depth_stencil: Option<DepthStencilState>,
  pub multisample: MultisampleState,
}

pub(crate) struct PipelineShaderGraphPair {
  vertex: Box<dyn ShaderAPI>,
  fragment: Box<dyn ShaderAPI>,
  current: Option<ShaderStages>,
}

thread_local! {
  static IN_BUILDING_SHADER_GRAPH: RefCell<Option<PipelineShaderGraphPair>> = RefCell::new(None);
}

pub struct ForNodes {
  pub item_node: ShaderGraphNodeRawHandle,
  pub index_node: ShaderGraphNodeRawHandle,
  pub for_cx: ShaderGraphNodeRawHandle,
}

pub(crate) fn modify_graph<T>(modifier: impl FnOnce(&mut dyn ShaderAPI) -> T) -> T {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| {
    let graph = graph.as_mut().unwrap();
    let graph = match graph.current.unwrap() {
      ShaderStages::Vertex => &mut graph.vertex,
      ShaderStages::Fragment => &mut graph.fragment,
    }
    .as_mut();

    modifier(graph)
  })
}

pub(crate) fn set_current_building(current: Option<ShaderStages>) {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| {
    let graph = graph.as_mut().unwrap();
    graph.current = current
  })
}

pub(crate) fn get_current_stage() -> Option<ShaderStages> {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| graph.as_mut().unwrap().current)
}

pub(crate) fn set_build_graph(vertex: Box<dyn ShaderAPI>, fragment: Box<dyn ShaderAPI>) {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| {
    graph.replace(PipelineShaderGraphPair {
      vertex,
      fragment,
      current: None,
    });
  })
}

pub(crate) fn take_build_graph() -> PipelineShaderGraphPair {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| graph.take().unwrap())
}
