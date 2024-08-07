use crate::*;

pub struct ShaderBindGroupBuilder {
  pub bindings: Vec<ShaderBindGroup>,
  pub current_index: usize,
}

impl Default for ShaderBindGroupBuilder {
  fn default() -> Self {
    Self {
      bindings: vec![Default::default(); 5],
      current_index: 0,
    }
  }
}

#[derive(Clone)]
pub struct BindingPreparer<T: ?Sized> {
  phantom: PhantomData<T>,
  entry: ShaderBindEntry,
}

impl<T: ShaderNodeType + ?Sized> BindingPreparer<T> {
  pub fn using(&self) -> Node<T> {
    let node = match get_current_stage().unwrap() {
      ShaderStages::Vertex => self.entry.vertex_node,
      ShaderStages::Fragment => self.entry.fragment_node,
      ShaderStages::Compute => self.entry.compute_node,
    };

    unsafe { node.into_node() }
  }

  pub fn using_graphics_pair(
    self,
    builder: &mut ShaderRenderPipelineBuilder,
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

impl ShaderBindGroupBuilder {
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn binding_dyn(&mut self, desc: ShaderBindingDescriptor) -> ShaderBindEntry {
    let bindgroup_index = self.current_index;
    let bindgroup = &mut self.bindings[bindgroup_index];

    let entry_index = bindgroup.bindings.len();

    let node = ShaderInputNode::Binding {
      desc: desc.clone(),
      bindgroup_index,
      entry_index,
    };

    let current_stage = get_current_stage();

    set_current_building(ShaderStages::Vertex.into());
    let vertex_node = node.clone().insert_api_raw();

    set_current_building(ShaderStages::Fragment.into());
    let fragment_node = node.clone().insert_api_raw();

    set_current_building(ShaderStages::Compute.into());
    let compute_node = node.insert_api_raw();

    set_current_building(current_stage);

    let entry = ShaderBindEntry {
      desc,
      vertex_node,
      fragment_node,
      compute_node,
    };

    bindgroup.bindings.push(entry.clone());

    entry
  }

  pub fn bind_by_and_prepare<T: ShaderBindingProvider>(
    &mut self,
    instance: &T,
  ) -> BindingPreparer<T::Node> {
    let entry = self.binding_dyn(instance.binding_desc());

    BindingPreparer {
      phantom: Default::default(),
      entry,
    }
  }

  /// use in current stage directly
  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> Node<T::Node> {
    self.bind_by_and_prepare(instance).using()
  }
}
