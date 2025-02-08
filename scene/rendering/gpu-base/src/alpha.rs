use crate::*;

pub struct ShaderAlphaConfig {
  pub alpha_mode: AlphaMode,
  pub alpha_cutoff: Node<f32>,
  pub alpha: Node<f32>,
}

impl ShaderAlphaConfig {
  pub fn apply(&self, builder: &mut ShaderFragmentBuilderView) {
    match self.alpha_mode {
      AlphaMode::Opaque => {}
      AlphaMode::Mask => {
        let alpha = self
          .alpha
          .less_than(self.alpha_cutoff)
          .select(val(0.), self.alpha);
        builder.register::<AlphaChannel>(alpha);
        builder.register::<AlphaCutChannel>(self.alpha_cutoff);
      }
      AlphaMode::Blend => {
        builder.register::<AlphaChannel>(self.alpha);
        builder.frag_output.iter_mut().for_each(|p| {
          p.states.blend = BlendState::ALPHA_BLENDING.into();
        });
      }
    };
  }
}
