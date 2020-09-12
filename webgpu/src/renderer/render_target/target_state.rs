use crate::texture_format::TextureFormat;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TargetStates {
  pub color_states: Vec<wgpu::ColorStateDescriptor>,
  pub depth_state: Option<wgpu::DepthStencilStateDescriptor>,
}

pub struct ColorStateModifier<'a> {
  state: &'a mut wgpu::ColorStateDescriptor,
}

impl<'a> ColorStateModifier<'a> {
  pub fn color_blend(&mut self, blend: wgpu::BlendDescriptor) {
    self.state.color_blend = blend;
  }
}

impl TargetStates {
  pub fn nth_color(&mut self, i: usize, visitor: impl Fn(&mut ColorStateModifier)) -> &mut Self {
    let mut modifier = ColorStateModifier {
      state: &mut self.color_states[i],
    };
    visitor(&mut modifier);
    self
  }

  pub fn first_color(&mut self, visitor: impl Fn(&mut ColorStateModifier)) -> &mut Self {
    self.nth_color(0, visitor)
  }
}

impl Default for TargetStates {
  fn default() -> Self {
    Self {
      color_states: vec![wgpu::ColorStateDescriptor {
        format: TextureFormat::Rgba8UnormSrgb.get_wgpu_format(),
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      }],
      depth_state: None,
    }
  }
}

impl AsRef<Self> for TargetStates {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl AsMut<Self> for TargetStates {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}
