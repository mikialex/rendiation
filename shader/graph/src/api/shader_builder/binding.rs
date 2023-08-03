use crate::*;

/// should impl by user's container ty
pub trait ShaderBindingProvider {
  type Node: ShaderGraphNodeType;
  fn binding_desc() -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      ty: Self::Node::TYPE,
    }
  }
}

#[derive(Clone, Copy)]
pub struct ShaderBindingDescriptor {
  pub should_as_storage_buffer_if_is_buffer_like: bool,
  pub ty: ShaderValueType,
}

impl<'a, T: ShaderBindingProvider> ShaderBindingProvider for &'a T {
  type Node = T::Node;

  fn binding_desc() -> ShaderBindingDescriptor {
    T::binding_desc()
  }
}

/// https://www.w3.org/TR/webgpu/#texture-format-caps
/// not all format could be filtered, use this to override
pub struct DisableFiltering<T>(pub T);

impl<T: ShaderBindingProvider> ShaderBindingProvider for DisableFiltering<T> {
  type Node = T::Node;
  fn binding_desc() -> ShaderBindingDescriptor {
    let mut ty = T::binding_desc();
    ty.ty.mutate_single(|ty| {
      if let ShaderValueSingleType::Texture {
        sample_type: TextureSampleType::Float { filterable },
        ..
      } = ty
      {
        *filterable = false;
      }

      if let ShaderValueSingleType::Sampler(ty) = ty {
        *ty = SamplerBindingType::NonFiltering
      }
    });

    ty
  }
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

  pub(crate) fn binding_ty_inner<T: ShaderBindingProvider>(
    &mut self,
  ) -> UniformNodePreparer<T::Node> {
    let bindgroup_index = self.current_index;
    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();
    let desc = T::binding_desc();

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
      desc,
      vertex_node,
      fragment_node,
    };

    bindgroup.bindings.push(entry);

    UniformNodePreparer {
      phantom: Default::default(),
      entry,
    }
  }

  pub fn binding<T: ShaderBindingProvider>(&mut self) -> UniformNodePreparer<T::Node> {
    self.binding_ty_inner::<T>()
  }

  pub fn bind_by<T: ShaderBindingProvider>(
    &mut self,
    _instance: &T,
  ) -> UniformNodePreparer<T::Node> {
    self.binding::<T>()
  }

  pub(crate) fn wrap(&mut self) -> ShaderGraphBindGroupDirectBuilder {
    ShaderGraphBindGroupDirectBuilder { builder: self }
  }
}

pub struct ShaderGraphBindGroupDirectBuilder<'a> {
  builder: &'a mut ShaderGraphBindGroupBuilder,
}

impl<'a> ShaderGraphBindGroupDirectBuilder<'a> {
  pub fn binding<T: ShaderBindingProvider>(&mut self) -> Node<T::Node> {
    self.builder.binding_ty_inner::<T>().using()
  }

  pub fn bind_by<T: ShaderBindingProvider>(&mut self, _instance: &T) -> Node<T::Node> {
    self.binding::<T>()
  }
}
