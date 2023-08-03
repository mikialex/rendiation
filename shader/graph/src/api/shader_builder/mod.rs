use crate::*;

pub mod binding;
pub use binding::*;
pub mod vertex;
pub use vertex::*;
pub mod fragment;
pub use fragment::*;
pub mod re_export;
pub use re_export::*;
pub mod builtin;
pub use builtin::*;

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

impl Default for ShaderGraphRenderPipelineBuilder {
  fn default() -> Self {
    set_build_graph();
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

  pub fn build<T: ShaderGraphCodeGenTarget>(
    mut self,
    target: T,
  ) -> Result<ShaderGraphCompileResult<T>, ShaderGraphBuildError> {
    self.vertex.sync_fragment_out(&mut self.fragment);

    let PipelineShaderGraphPair {
      mut vertex,
      mut fragment,
      ..
    } = take_build_graph();

    vertex.top_scope_mut().resolve_all_pending();
    fragment.top_scope_mut().resolve_all_pending();

    let shader = target.compile(&self, vertex, fragment);

    Ok(ShaderGraphCompileResult {
      shader,
      target,
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

  fn build_self(&self) -> Result<ShaderGraphRenderPipelineBuilder, ShaderGraphBuildError> {
    let mut builder = Default::default();
    self.build(&mut builder)?;
    self.post_build(&mut builder)?;
    Ok(builder)
  }
}

impl GraphicsShaderProvider for () {}

pub struct ShaderGraphCompileResult<T: ShaderGraphCodeGenTarget> {
  pub target: T,
  pub shader: T::ShaderSource,
  pub bindings: ShaderGraphBindGroupBuilder,
  pub vertex_layouts: Vec<ShaderGraphVertexBufferLayout>,
  pub primitive_state: PrimitiveState,
  pub color_states: Vec<ColorTargetState>,
  pub depth_stencil: Option<DepthStencilState>,
  pub multisample: MultisampleState,
}

#[derive(Default)]
pub struct SemanticRegistry {
  registered: FastHashMap<TypeId, Node<AnyType>>,
}

impl SemanticRegistry {
  pub fn query_typed_both_stage<T: SemanticFragmentShaderValue + SemanticFragmentShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderGraphBuildError> {
    self
      .query(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { std::mem::transmute(n) })
  }

  pub fn register_typed_both_stage<T: SemanticVertexShaderValue + SemanticFragmentShaderValue>(
    &mut self,
    node: impl Into<Node<<T as SemanticVertexShaderValue>::ValueType>>,
  ) {
    self.register(TypeId::of::<T>(), node.into().cast_untyped_node());
  }

  pub fn query(
    &self,
    id: TypeId,
    name: &'static str,
  ) -> Result<Node<AnyType>, ShaderGraphBuildError> {
    self
      .registered
      .get(&id)
      .copied()
      .ok_or(ShaderGraphBuildError::MissingRequiredDependency(name))
  }

  pub fn register(&mut self, id: TypeId, node: NodeUntyped) -> &Node<AnyType> {
    self.registered.insert(id, node);
    // fixme, rust hashmap, pain in the ass..
    self.registered.get(&id).unwrap()
  }
}

#[derive(Default)]
pub(crate) struct PipelineShaderGraphPair {
  vertex: ShaderGraphBuilder,
  fragment: ShaderGraphBuilder,
  current: Option<ShaderStages>,
}

thread_local! {
  static IN_BUILDING_SHADER_GRAPH: RefCell<Option<PipelineShaderGraphPair>> = RefCell::new(None);
}

pub(crate) fn modify_graph<T>(modifier: impl FnOnce(&mut ShaderGraphBuilder) -> T) -> T {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| {
    let graph = graph.as_mut().unwrap();
    let graph = match graph.current.unwrap() {
      ShaderStages::Vertex => &mut graph.vertex,
      ShaderStages::Fragment => &mut graph.fragment,
    };

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

pub(crate) fn set_build_graph() {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| {
    graph.replace(Default::default());
  })
}

pub(crate) fn take_build_graph() -> PipelineShaderGraphPair {
  IN_BUILDING_SHADER_GRAPH.with_borrow_mut(|graph| graph.take().unwrap())
}
