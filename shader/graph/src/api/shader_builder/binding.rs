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
  pub(crate) fn uniform_ty_inner<T, N: ShaderGraphNodeType>(
    &mut self,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<N> {
    let bindgroup_index = index.into();
    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let ty = N::TYPE;

    let node = ShaderGraphInputNode::Uniform {
      bindgroup_index,
      entry_index,
    };

    let current_stage = get_current_stage();

    set_current_building(ShaderStages::Vertex.into());
    let vertex_node = node.clone().insert_graph::<N>().handle();

    set_current_building(ShaderStages::Fragment.into());
    let fragment_node = node.insert_graph::<N>().handle();

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
    self.uniform_ty_inner::<T, T::Node>(index)
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want toc cast
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
    Ok(self.uniform_ty_inner::<T, N>(index))
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
    self.builder.uniform_ty_inner::<T, T::Node>(index).using()
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
    index: impl Into<usize>,
  ) -> Node<T::Node> {
    self.uniform::<T>(index)
  }

  /// N: the node type you want toc cast
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
    Ok(self.builder.uniform_ty_inner::<T, N>(index).using())
  }
}
