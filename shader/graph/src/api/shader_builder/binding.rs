use crate::*;

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
pub struct DisableFiltering<T>(pub T);

impl<T: ShaderUniformProvider> ShaderUniformProvider for DisableFiltering<T> {
  type Node = T::Node;

  fn modify_node_shader_value_type(ty: &mut ShaderValueType) {
    if let ShaderValueType::Texture {
      sample_type: TextureSampleType::Float { filterable },
      ..
    } = ty
    {
      *filterable = false;
    }

    if let ShaderValueType::Sampler(ty) = ty {
      *ty = SamplerBindingType::NonFiltering
    }
  }
}

/// should impl by user's container ty
pub trait DynamicShaderUniformProvider {
  fn to_value(&self) -> ShaderValueType;
}

pub struct ShaderGraphBindGroupBuilder {
  pub bindings: Vec<ShaderGraphBindGroup>,
  pub current_index: usize,
}

impl Default for ShaderGraphBindGroupBuilder {
  fn default() -> Self {
    Self {
      bindings: vec![Default::default(); 5],
      current_index: 0,
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
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub(crate) fn uniform_ty_inner<T: ShaderUniformProvider>(
    &mut self,
  ) -> UniformNodePreparer<T::Node> {
    let bindgroup_index = self.current_index;
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

  pub fn uniform<T: ShaderUniformProvider>(&mut self) -> UniformNodePreparer<T::Node> {
    self.uniform_ty_inner::<T>()
  }

  pub fn uniform_by<T: ShaderUniformProvider>(
    &mut self,
    _instance: &T,
  ) -> UniformNodePreparer<T::Node> {
    self.uniform::<T>()
  }

  /// N: the node type you want to cast
  pub fn uniform_dyn_ty_by<T, N>(
    &mut self,
    instance: &T,
  ) -> Result<UniformNodePreparer<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.uniform_ty_inner::<DirectProvider<N>>())
  }

  pub(crate) fn wrap(&mut self) -> ShaderGraphBindGroupDirectBuilder {
    ShaderGraphBindGroupDirectBuilder { builder: self }
  }
}

pub struct ShaderGraphBindGroupDirectBuilder<'a> {
  builder: &'a mut ShaderGraphBindGroupBuilder,
}

impl<'a> ShaderGraphBindGroupDirectBuilder<'a> {
  pub fn uniform<T: ShaderUniformProvider>(&mut self) -> Node<T::Node> {
    self.builder.uniform_ty_inner::<T>().using()
  }

  pub fn uniform_by<T: ShaderUniformProvider>(&mut self, _instance: &T) -> Node<T::Node> {
    self.uniform::<T>()
  }

  /// N: the node type you want to cast
  pub fn uniform_dyn_ty_by<T, N>(&mut self, instance: &T) -> Result<Node<N>, ShaderGraphBuildError>
  where
    T: DynamicShaderUniformProvider,
    N: ShaderGraphNodeType,
  {
    if instance.to_value() != N::TYPE {
      return Err(ShaderGraphBuildError::FailedDowncastShaderValueFromInput);
    }
    Ok(self.builder.uniform_ty_inner::<DirectProvider<N>>().using())
  }
}
