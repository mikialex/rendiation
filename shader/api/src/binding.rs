use crate::*;

pub struct ShaderBindGroupBuilder {
  pub bindings: Vec<ShaderBindGroup>,
  pub current_index: usize,
  pub custom_states: FastHashMap<String, Arc<dyn Any>>,
  binding_re_enter: BindingReEnter,
}

enum BindingReEnter {
  None,
  Recording(Vec<(usize, usize)>),
  ReEnter(Vec<(usize, usize)>, usize),
}

impl Default for ShaderBindGroupBuilder {
  fn default() -> Self {
    Self {
      bindings: vec![Default::default(); 5],
      current_index: 0,
      custom_states: Default::default(),
      binding_re_enter: BindingReEnter::None,
    }
  }
}

pub struct BindingPreparer<'a, T> {
  source: &'a T,
  builder: &'a mut ShaderRenderPipelineBuilder,
  bind_info: Option<Vec<(usize, usize)>>,
}

impl<T: AbstractShaderBindingSource> BindingPreparer<'_, T> {
  pub fn using(&mut self) -> T::ShaderBindResult {
    self.builder.binding_re_enter = if let Some(bind_info) = &self.bind_info {
      BindingReEnter::ReEnter(bind_info.clone(), 0)
    } else {
      BindingReEnter::Recording(Default::default())
    };
    let r = self.source.bind_shader(self.builder);

    if let BindingReEnter::Recording(info) = &mut self.builder.binding_re_enter {
      self.bind_info = Some(info.clone());
    }

    self.builder.binding_re_enter = BindingReEnter::None;

    r
  }

  pub fn using_graphics_pair(
    mut self,
    register: impl Fn(&mut SemanticRegistry, &T::ShaderBindResult),
  ) -> GraphicsPairInputNodeAccessor<T> {
    assert!(
      get_current_stage().is_none(),
      "using_graphics_pair must be called outside any graphics sub shader stage"
    );
    set_current_building(ShaderStage::Vertex.into());
    let vertex = self.using();
    register(&mut self.builder.vertex.registry, &vertex);
    set_current_building(ShaderStage::Fragment.into());
    let fragment = self.using();
    register(&mut self.builder.fragment.registry, &fragment);
    set_current_building(None);
    GraphicsPairInputNodeAccessor { vertex, fragment }
  }
}

pub struct GraphicsPairInputNodeAccessor<T: AbstractShaderBindingSource> {
  pub vertex: T::ShaderBindResult,
  pub fragment: T::ShaderBindResult,
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

pub trait AbstractShaderBindingSource {
  type ShaderBindResult;
  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult;
}
impl<T: ShaderBindingProvider> AbstractShaderBindingSource for T {
  type ShaderBindResult = T::ShaderInstance;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    ctx.bind_single_by(self)
  }
}

impl ShaderBindGroupBuilder {
  pub fn set_binding_slot(&mut self, new: usize) -> usize {
    std::mem::replace(&mut self.current_index, new)
  }

  pub fn bind_by<T: AbstractShaderBindingSource>(&mut self, instance: &T) -> T::ShaderBindResult {
    instance.bind_shader(self)
  }

  pub fn bind_single_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    let node = self.binding_dyn(instance.binding_desc()).using();
    instance.create_instance(unsafe { node.into_node() })
  }

  pub fn binding_dyn(&mut self, desc: ShaderBindingDescriptor) -> &mut ShaderBindEntry {
    if let BindingReEnter::ReEnter(info, counter) = &mut self.binding_re_enter {
      let (bindgroup_index, entry_index) = info[*counter];
      *counter += 1;
      let bindgroup = &mut self.bindings[bindgroup_index];
      &mut bindgroup.bindings[entry_index]
    } else {
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

      if let BindingReEnter::Recording(info) = &mut self.binding_re_enter {
        info.push((bindgroup_index, entry_index));
      }

      bindgroup.bindings.push(entry.clone());

      bindgroup.bindings.last_mut().unwrap()
    }
  }
}

impl ShaderRenderPipelineBuilder {
  pub fn bind_by_and_prepare<'a, T: AbstractShaderBindingSource>(
    &'a mut self,
    instance: &'a T,
  ) -> BindingPreparer<'a, T> {
    BindingPreparer {
      source: instance,
      builder: self,
      bind_info: None,
    }
  }

  /// use in current stage directly
  pub fn bind_by<T: ShaderBindingProvider>(&mut self, instance: &T) -> T::ShaderInstance {
    self.bind_by_and_prepare(instance).using()
  }
}
