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

pub struct BindingPreparer<'a, T> {
  source: &'a T,
  builder: &'a mut ShaderRenderPipelineBuilder,
  bindgroup_index: usize,
}

impl<T: ShaderBindingProvider> BindingPreparer<'_, T> {
  pub fn using(&mut self) -> T::ShaderInstance {
    let entry = self.builder.bindings[self.bindgroup_index]
      .bindings
      .last_mut()
      .unwrap();
    let node = entry.using();
    self.source.create_instance(unsafe { node.into_node() })
  }

  pub fn using_graphics_pair(
    mut self,
    register: impl Fn(&mut SemanticRegistry, T::ShaderInstance),
  ) -> GraphicsPairInputNodeAccessor<T> {
    assert!(
      get_current_stage().is_none(),
      "using_graphics_pair must be called outside any graphics sub shader stage"
    );
    set_current_building(ShaderStage::Vertex.into());
    let vertex = self.using();
    register(&mut self.builder.vertex.registry, vertex.clone());
    set_current_building(ShaderStage::Fragment.into());
    let fragment = self.using();
    register(&mut self.builder.fragment.registry, fragment.clone());
    set_current_building(None);
    GraphicsPairInputNodeAccessor { vertex, fragment }
  }
}

pub struct GraphicsPairInputNodeAccessor<T: ShaderBindingProvider> {
  pub vertex: T::ShaderInstance,
  pub fragment: T::ShaderInstance,
}

impl<T: ShaderBindingProvider> GraphicsPairInputNodeAccessor<T> {
  pub fn get(&self) -> T::ShaderInstance {
    match get_current_stage() {
      Some(ShaderStage::Vertex) => self.vertex.clone(),
      Some(ShaderStage::Fragment) => self.fragment.clone(),
      _ => unreachable!("expect in graphics stage"),
    }
  }
}

impl ShaderBindGroupBuilder {
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    let node = self.binding_dyn(instance.binding_desc()).using();
    instance.create_instance(unsafe { node.into_node() })
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
  pub fn bind_by_and_prepare<'a, T: ShaderBindingProvider>(
    &'a mut self,
    instance: &'a T,
  ) -> BindingPreparer<'a, T> {
    let desc = instance.binding_desc();
    self.binding_dyn(desc);

    BindingPreparer {
      source: instance,
      bindgroup_index: self.current_index,
      builder: self,
    }
  }

  /// use in current stage directly
  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    self.bind_by_and_prepare(instance).using()
  }
}
