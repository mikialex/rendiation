use crate::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RenderTargetFormatsInfo {
  pub color: Vec<TextureFormat>,
  pub depth: Option<TextureFormat>,
}

#[derive(Clone, PartialEq)]
pub struct TargetStates {
  pub color_states: Vec<ColorTargetState>,
  pub depth_state: Option<DepthStencilState>,
}

pub struct ColorTargetStateModifier<'a> {
  state: &'a mut ColorTargetState,
}

impl<'a> ColorTargetStateModifier<'a> {
  pub fn color_blend(&mut self, blend: BlendState) {
    self.state.color_blend = blend;
  }
}

impl TargetStates {
  pub fn nth_color(
    &mut self,
    i: usize,
    visitor: impl Fn(&mut ColorTargetStateModifier),
  ) -> &mut Self {
    let mut modifier = ColorTargetStateModifier {
      state: &mut self.color_states[i],
    };
    visitor(&mut modifier);
    self
  }

  pub fn first_color(&mut self, visitor: impl Fn(&mut ColorTargetStateModifier)) -> &mut Self {
    self.nth_color(0, visitor)
  }
}

impl Default for TargetStates {
  fn default() -> Self {
    Self {
      color_states: vec![ColorTargetState {
        format: TextureFormat::Rgba8UnormSrgb,
        color_blend: BlendState::REPLACE,
        alpha_blend: BlendState::REPLACE,
        write_mask: ColorWrite::ALL,
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
