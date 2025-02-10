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

pub struct BindingPreparer<'a, T: ?Sized> {
  phantom: PhantomData<T>,
  builder: &'a mut ShaderRenderPipelineBuilder,
  bindgroup_index: usize,
}

impl<T: ShaderNodeType + ?Sized> BindingPreparer<'_, T> {
  pub fn using(&mut self) -> Node<T> {
    let entry = self.builder.bindings[self.bindgroup_index]
      .bindings
      .last_mut()
      .unwrap();
    let node = entry.using();
    unsafe { node.into_node() }
  }

  pub fn using_graphics_pair(
    mut self,
    register: impl Fn(&mut SemanticRegistry, Node<T>),
  ) -> GraphicsPairInputNodeAccessor<T> {
    assert!(
      get_current_stage().is_none(),
      "using_graphics_pair must be called outside any graphics sub shader stage"
    );
    set_current_building(ShaderStage::Vertex.into());
    let vertex = self.using();
    register(&mut self.builder.vertex.registry, vertex);
    set_current_building(ShaderStage::Fragment.into());
    let fragment = self.using();
    register(&mut self.builder.fragment.registry, fragment);
    set_current_building(None);
    GraphicsPairInputNodeAccessor { vertex, fragment }
  }
}

pub struct GraphicsPairInputNodeAccessor<T: ?Sized> {
  pub vertex: Node<T>,
  pub fragment: Node<T>,
}

impl<T> GraphicsPairInputNodeAccessor<T> {
  pub fn get(&self) -> Node<T> {
    match get_current_stage() {
      Some(ShaderStage::Vertex) => self.vertex,
      Some(ShaderStage::Fragment) => self.fragment,
      _ => unreachable!("expect in graphics stage"),
    }
  }
}

impl ShaderBindGroupBuilder {
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> Node<T::Node> {
    unsafe {
      self
        .binding_dyn(instance.binding_desc())
        .using()
        .into_node()
    }
  }

  pub fn binding_dyn(&mut self, desc: ShaderBindingDescriptor) -> &mut ShaderBindEntry {
    let bindgroup_index = self.current_index;

    let bindgroup = &mut self.bindings[bindgroup_index];
    let entry_index = bindgroup.bindings.len();

    let entry = ShaderBindEntry {
      desc,
      vertex_node: None,
      fragment_node: None,
      compute_node: None,
      visibility: ShaderStages::empty(),
      entry_index,
      bindgroup_index,
    };

    bindgroup.bindings.push(entry.clone());

    bindgroup.bindings.last_mut().unwrap()
  }
}

impl ShaderRenderPipelineBuilder {
  pub fn bind_by_and_prepare<T: ShaderBindingProvider>(
    &mut self,
    instance: &T,
  ) -> BindingPreparer<T::Node> {
    let desc = instance.binding_desc();
    self.binding_dyn(desc);

    BindingPreparer {
      phantom: Default::default(),
      bindgroup_index: self.current_index,
      builder: self,
    }
  }

  /// use in current stage directly
  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> Node<T::Node> {
    self.bind_by_and_prepare(instance).using()
  }
}
