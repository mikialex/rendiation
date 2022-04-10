use std::sync::Mutex;

use shadergraph::ShaderGraphFragmentBuilder;

use crate::ValueIDGenerator;

pub static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<MaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone)]
pub struct MaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: wgpu::CompareFunction,
  pub stencil: wgpu::StencilState,
  pub bias: wgpu::DepthBiasState,
  pub blend: Option<wgpu::BlendState>,
  pub write_mask: wgpu::ColorWrites,
}

impl PartialEq for MaterialStates {
  fn eq(&self, other: &Self) -> bool {
    self.depth_write_enabled == other.depth_write_enabled
      && self.depth_compare == other.depth_compare
      && self.stencil == other.stencil
      && self.bias == other.bias
      && self.blend == other.blend
      && self.write_mask == other.write_mask
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
  }
}

impl Eq for MaterialStates {}

impl MaterialStates {
  pub fn map_color_states(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
      format,
      blend: self.blend,
      write_mask: self.write_mask,
    }
  }
  pub fn map_depth_stencil_state(
    &self,
    format: Option<wgpu::TextureFormat>,
  ) -> Option<wgpu::DepthStencilState> {
    format.map(|format| wgpu::DepthStencilState {
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
      depth_compare: wgpu::CompareFunction::Less,
      blend: None,
      write_mask: wgpu::ColorWrites::all(),
      bias: Default::default(),
      stencil: Default::default(),
    }
  }
}
