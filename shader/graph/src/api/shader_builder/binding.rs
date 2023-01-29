use crate::*;

#[derive(Clone, Copy, Hash)]
pub enum SemanticBinding {
  Global,
  Camera,
  Pass,
  Material,
  Object,
}

/// simple and wonderful
pub type SB = SemanticBinding;

impl SemanticBinding {
  pub fn binding_index(&self) -> usize {
    match self {
      SemanticBinding::Global => 4,
      SemanticBinding::Camera => 3,
      SemanticBinding::Pass => 2,
      SemanticBinding::Material => 1,
      SemanticBinding::Object => 0,
    }
  }
}

impl From<SB> for usize {
  fn from(v: SB) -> Self {
    v.binding_index()
  }
}

/// should impl by user's container ty
pub trait ShaderUniformProvider {
  type Node: ShaderGraphNodeType;
  // provide a way to modify node shader ty
  fn modify_node_shader_value_type(_ty: &mut ShaderValueType) {
    // default do nothing
  }
}

impl<'a, T: ShaderUniformProvider> ShaderUniformProvider for &'a T {
  type Node = T::Node;

  fn modify_node_shader_value_type(ty: &mut ShaderValueType) {
    T::modify_node_shader_value_type(ty)
  }
}

struct DirectProvider<N>(PhantomData<N>);
impl<N: ShaderGraphNodeType> ShaderUniformProvider for DirectProvider<N> {
  type Node = N;
}

/// https://www.w3.org/TR/webgpu/#texture-format-caps
/// not all format could be filtered, use this to override
pub struct DisablePossibleFiltering<T>(pub T);

impl<T: ShaderUniformProvider> ShaderUniformProvider for DisablePossibleFiltering<T> {
  type Node = T::Node;

  fn modify_node_shader_value_type(ty: &mut ShaderValueType) {
    if let ShaderValueType::Texture {
      sample_type: TextureSampleType::Float { filterable },
      ..
    } = ty
    {
      *filterable = false;
    }
  }
}

/// should impl by user's container ty
pub trait DynamicShaderUniformProvider {
  fn to_value(&self) -> ShaderValueType;
}

pub struct ShaderGraphBindGroupBuilder {
  pub bindings: Vec<ShaderGraphBindGroup>,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      bindings: vec![Default::default(); 5],
    }
  }
}

#[derive(Clone)]
pub struct UniformNodePreparer<T> {
  phantom: PhantomData<T>,
  entry: ShaderGraphBindEntry,
}

impl<T: ShaderGraphNodeType> UniformNodePreparer<T> {
  pub fn using(&self) -> Node<T> {
    let node = match get_current_stage().unwrap() {
      ShaderStages::Vertex => self.entry.vertex_node,
      ShaderStages::Fragment => self.entry.fragment_node,
    };

    unsafe { node.into_node() }
  }

  #[must_use]
  pub fn using_both(
    self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    register: impl Fn(&mut SemanticRegistry, Node<T>),
  ) -> Self {
    unsafe {
      set_current_building(ShaderStages::Vertex.into());
      register(
        &mut builder.vertex.registry,
        self.entry.vertex_node.into_node(),
      );
      set_current_building(ShaderStages::Fragment.into());
      register(
        &mut builder.fragment.registry,
        self.entry.fragment_node.into_node(),
      );
      set_current_building(None);
    }
    self
  }
}

impl ShaderGraphBindGroupBuilder {
  pub(crate) fn uniform_ty_inner<T: ShaderUniformProvider>(
    &mut self,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    let bindgroup_index = index.into();
    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let mut ty = T::Node::TYPE;
    T::modify_node_shader_value_type(&mut ty);

    let node = ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    };

    let current_stage = get_current_stage();

    set_current_building(ShaderStages::Vertex.into());
    let vertex_node = node.clone().insert_graph::<T::Node>().handle();

    set_current_building(ShaderStages::Fragment.into());
    let fragment_node = node.insert_graph::<T::Node>().handle();

    set_current_building(current_stage);

    let entry = ShaderGraphBindEntry {
      ty,
      vertex_node,
      fragment_node,
    };

    bindgroup.bindings.push(entry);

    UniformNodePreparer {
      phantom: Default::default(),
      entry,
    }
  }

  pub fn uniform<T: ShaderUniformProvider>(
    &mut self,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform_ty_inner::<T>(index)
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want to cast
  pub fn uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
    index: impl Into<usize>,
  ) -> Result<UniformNodePreparer<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.uniform_ty_inner::<DirectProvider<N>>(index))
  }

  pub(crate) fn wrap(&mut self) -> ShaderGraphBindGroupDirectBuilder {
    ShaderGraphBindGroupDirectBuilder { builder: self }
  }
}

pub struct ShaderGraphBindGroupDirectBuilder<'a> {
  builder: &'a mut ShaderGraphBindGroupBuilder,
}

impl<'a> ShaderGraphBindGroupDirectBuilder<'a> {
  pub fn uniform<T: ShaderUniformProvider>(&mut self, index: impl Into<usize>) -> Node<T::Node> {
    self.builder.uniform_ty_inner::<T>(index).using()
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> Node<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want to cast
  pub fn uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
    index: impl Into<usize>,
  ) -> Result<Node<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(
      self
        .builder
        .uniform_ty_inner::<DirectProvider<N>>(index)
        .using(),
    )
  }
}
