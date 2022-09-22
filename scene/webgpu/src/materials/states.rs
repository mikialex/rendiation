use crate::*;

#[derive(Debug, Clone)]
pub struct MaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: webgpu::CompareFunction,
  pub stencil: webgpu::StencilState,
  pub bias: webgpu::DepthBiasState,
  pub blend: Option<webgpu::BlendState>,
  pub write_mask: webgpu::ColorWrites,
  pub front_face: FrontFace,
  pub cull_mode: Option<Face>,
}

/// manually impl because lint complains
impl PartialEq for MaterialStates {
  fn eq(&self, other: &Self) -> bool {
    self.depth_write_enabled == other.depth_write_enabled
      && self.depth_compare == other.depth_compare
      && self.stencil == other.stencil
      && self.bias == other.bias
      && self.blend == other.blend
      && self.write_mask == other.write_mask
      && self.front_face == other.front_face
      && self.cull_mode == other.cull_mode
  }
}

impl std::hash::Hash for MaterialStates {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.depth_write_enabled.hash(state);
    self.depth_compare.hash(state);
    self.stencil.hash(state);
    self.bias.slope_scale.to_bits().hash(state);
    self.bias.clamp.to_bits().hash(state);
    self.bias.constant.hash(state);
    self.blend.hash(state);
    self.write_mask.hash(state);
    self.front_face.hash(state);
    self.cull_mode.hash(state);
  }
}

impl Eq for MaterialStates {}

impl MaterialStates {
  pub fn map_color_states(&self, format: webgpu::TextureFormat) -> webgpu::ColorTargetState {
    webgpu::ColorTargetState {
      format,
      blend: self.blend,
      write_mask: self.write_mask,
    }
  }
  pub fn map_depth_stencil_state(
    &self,
    format: Option<webgpu::TextureFormat>,
  ) -> Option<webgpu::DepthStencilState> {
    format.map(|format| webgpu::DepthStencilState {
      format,
      depth_write_enabled: self.depth_write_enabled,
      depth_compare: self.depth_compare,
      stencil: self.stencil.clone(),
      bias: self.bias,
    })
  }

  pub fn apply_pipeline_builder(&self, builder: &mut ShaderGraphFragmentBuilder) {
    // override all outputs states
    builder.frag_output.iter_mut().for_each(|(_, state)| {
      let format = state.format;
      *state = self.map_color_states(format);
    });

    // and depth_stencil if they exist
    let format = builder.depth_stencil.as_ref().map(|s| s.format);
    builder.depth_stencil = self.map_depth_stencil_state(format);
  }
}

impl Default for MaterialStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: webgpu::CompareFunction::Less,
      blend: None,
      write_mask: webgpu::ColorWrites::all(),
      bias: Default::default(),
      stencil: Default::default(),
      front_face: FrontFace::Ccw,
      cull_mode: Some(Face::Back),
    }
  }
}

static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<MaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Clone)]
pub struct StateControl<T> {
  pub material: T,
  pub states: MaterialStates,
}

pub trait IntoStateControl: Sized {
  fn use_state(self) -> StateControl<Self> {
    StateControl {
      material: self,
      states: Default::default(),
    }
  }

  /// disable depth rw, double face
  fn use_state_helper_like(self) -> StateControl<Self> {
    let mut states = MaterialStates::default();
    states.depth_write_enabled = false;
    states.depth_compare = webgpu::CompareFunction::Always;
    states.cull_mode = None;
    StateControl {
      material: self,
      states,
    }
  }
}

impl<T> IntoStateControl for T {}

pub struct StateControlGPU<T: WebGPUMaterial> {
  state_id: Cell<ValueID<MaterialStates>>,
  gpu: T::GPU,
}

impl<T: WebGPUMaterial> ShaderHashProvider for StateControlGPU<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.state_id.get().hash(hasher);
    self.gpu.hash_pipeline(hasher);
  }
}

impl<T> ShaderPassBuilder for StateControlGPU<T>
where
  T: WebGPUMaterial,
{
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.gpu.setup_pass(ctx)
  }
}

impl<T: WebGPUMaterial> ShaderGraphProvider for StateControlGPU<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    let id = STATE_ID.lock().unwrap();

    let value = id.get_value(self.state_id.get()).unwrap();

    builder.vertex(|builder, _| {
      builder.primitive_state.front_face = value.front_face;
      builder.primitive_state.cull_mode = value.cull_mode;
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      value.apply_pipeline_builder(builder);
      Ok(())
    })?;
    self.gpu.build(builder)
  }
}

impl<T> WebGPUMaterial for StateControl<T>
where
  T: Clone,
  T: WebGPUMaterial,
{
  type GPU = StateControlGPU<T>;

  fn create_gpu(&self, ctx: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let gpu = self.material.create_gpu(ctx, gpu);

    let state_id = STATE_ID.lock().unwrap().get_uuid(&self.states);

    StateControlGPU {
      state_id: Cell::new(state_id),
      gpu,
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.material.is_keep_mesh_shape()
  }
  fn is_transparent(&self) -> bool {
    self.states.blend.is_some()
  }
  fn preferred_shading(&self) -> Option<&'static dyn LightableSurfaceShadingDyn> {
    self.material.preferred_shading()
  }
}
