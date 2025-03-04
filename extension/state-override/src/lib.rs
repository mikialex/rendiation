use std::hash::Hash;

use interning::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[derive(Debug, Clone)]
pub struct MaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: CompareFunction,
  pub stencil: StencilState,
  pub bias: DepthBiasState,
  pub blend: Option<BlendState>,
  pub write_mask: ColorWrites,
  pub front_face: FrontFace,
  pub cull_mode: Option<Face>,
}

impl Default for MaterialStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: CompareFunction::Less,
      blend: None,
      write_mask: ColorWrites::all(),
      bias: Default::default(),
      stencil: Default::default(),
      front_face: FrontFace::Ccw,
      cull_mode: Some(Face::Back),
    }
  }
}

impl MaterialStates {
  pub fn helper_like() -> Self {
    Self {
      depth_write_enabled: false,
      depth_compare: CompareFunction::Always,
      cull_mode: None,
      ..Default::default()
    }
  }
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
  pub fn map_color_states(&self, format: TextureFormat) -> ColorTargetState {
    let mut s = ColorTargetState {
      format,
      blend: self.blend,
      write_mask: self.write_mask,
    };

    if !is_texture_fmt_blendable(format) {
      s.blend = None;
    }

    s
  }
  pub fn map_depth_stencil_state(
    &self,
    format: Option<TextureFormat>,
  ) -> Option<DepthStencilState> {
    format.map(|format| DepthStencilState {
      format,
      depth_write_enabled: self.depth_write_enabled,
      depth_compare: self.depth_compare,
      stencil: self.stencil.clone(),
      bias: self.bias,
    })
  }

  pub fn apply_pipeline_builder(&self, builder: &mut ShaderFragmentBuilder) {
    // override all outputs states
    builder.frag_output.iter_mut().for_each(|p| {
      let format = p.states.format;
      p.states = self.map_color_states(format);
    });

    // and depth_stencil if they exist
    let format = builder.depth_stencil.as_ref().map(|s| s.format);
    builder.depth_stencil = self.map_depth_stencil_state(format);
  }
}

pub struct StateGPUImpl {
  state_id: InternedValue<MaterialStates>,
}

define_static_id_generator!(STATE_ID, MaterialStates);

impl StateGPUImpl {
  pub fn new(states: &MaterialStates) -> Self {
    let state_id = STATE_ID.lock().get_uuid(states);
    Self { state_id }
  }
}

impl ShaderHashProvider for StateGPUImpl {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.state_id.hash(hasher)
  }
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for StateGPUImpl {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let id = STATE_ID.lock();

    let value = id.get_value(self.state_id).unwrap();

    builder.vertex(|builder, _| {
      builder.primitive_state.front_face = value.front_face;
      builder.primitive_state.cull_mode = value.cull_mode;
    });

    builder.fragment(|builder, _| {
      value.apply_pipeline_builder(builder);
    })
  }
}
