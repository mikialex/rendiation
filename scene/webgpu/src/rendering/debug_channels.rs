use crate::*;

#[derive(Default)]
pub struct ScreenChannelDebugger {
  pub channels: Vec<Box<dyn ChannelVisualize>>,
}

impl ScreenChannelDebugger {
  pub fn default_useful() -> Self {
    Self::default()
      .push_debug_channel(FragmentWorldNormal)
      .push_debug_channel(FragmentUv)
      .push_debug_channel(ColorChannel)
  }
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
    builder.log_result = true;
    builder.fragment(|builder, _| {
      let ndc_position = builder.query::<FragmentPosition>()?;

      let output = consts(Vec4::new(0., 0., 0., 1.)).mutable();

      let width = builder.query::<RenderBufferSize>()?.x();

      let step = width / consts(self.channels.len() as f32);
      let start = consts(0.).mutable();
      for channel in &self.channels {
        let x = ndc_position.x();
        let start_current = start.get();
        let start_end = start_current + step;
        if_by(
          start_current
            .less_than(x)
            .and(x.less_or_equal_than(start_end)),
          || {
            output.set(output.get() + channel.to_screen(builder));
          },
        );
        start.set(start_end);
      }

      builder.set_fragment_out(0, output.get())
    })
  }
}

impl ChannelVisualize for FragmentWorldNormal {
  fn to_screen(&self, builder: &ShaderGraphFragmentBuilderView) -> Node<Vec4<f32>> {
    let normal = builder
      .query::<Self>()
      .unwrap_or_else(|_| consts(Vec3::zero()));

    (normal * consts(0.5) + consts(Vec3::splat(0.5)), 1.).into()
  }
}

impl ChannelVisualize for FragmentUv {
  fn to_screen(&self, builder: &ShaderGraphFragmentBuilderView) -> Node<Vec4<f32>> {
    let uv = builder
      .query::<Self>()
      .unwrap_or_else(|_| consts(Vec2::zero()));

    (uv, 0., 1.).into()
  }
}

impl ChannelVisualize for ColorChannel {
  fn to_screen(&self, builder: &ShaderGraphFragmentBuilderView) -> Node<Vec4<f32>> {
    let value = builder
      .query::<Self>()
      .unwrap_or_else(|_| consts(Vec3::zero()));

    (value, 1.).into()
  }
}