use crate::*;

#[derive(Default)]
pub struct ScreenChannelDebugger {
  pub channels: Vec<Box<dyn ChannelVisualize>>,
}

pub trait ChannelVisualize: Any {
  fn to_screen(&self, builder: &ShaderGraphFragmentBuilderView) -> Node<Vec4<f32>>;
}

impl ScreenChannelDebugger {
  pub fn push_debug_channel(mut self, channel: impl ChannelVisualize) -> Self {
    self.channels.push(Box::new(channel));
    self
  }
}

impl ShaderHashProvider for ScreenChannelDebugger {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.channels.iter().for_each(|c| c.type_id().hash(hasher))
  }
}

impl ShaderGraphProvider for ScreenChannelDebugger {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let ndc_position = builder.query::<FragmentNDCPosition>()?;

      let mut output = consts(Vec4::new(0., 0., 0., 1.)).mutable();

      let step = 2. / self.channels.len() as f32;
      let mut start = -1.;
      for channel in &self.channels {
        if_by(
          consts(start) <= ndc_position.x() && ndc_position <= consts(start + step),
          || {
            output.set(channel.to_screen(builder));
          },
        )
      }

      builder.set_fragment_out(0, output);

      Ok(())
    })
  }
}
