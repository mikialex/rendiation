use crate::*;

#[derive(Default)]
pub struct ScreenChannelDebugger {
  pub channels: Vec<Box<dyn ChannelVisualize>>,
}

impl ScreenChannelDebugger {
  pub fn default_useful() -> Self {
    Self::default()
      .push_debug_channel(FragmentRenderNormal)
      .push_debug_channel(FragmentUv)
      .push_debug_channel(ColorChannel)
      .push_debug_channel(RoughnessChannel)
      .push_debug_channel(MetallicChannel)
  }
}

pub trait ChannelVisualize: Any {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>>;
}

impl ScreenChannelDebugger {
  pub fn push_debug_channel(mut self, channel: impl ChannelVisualize) -> Self {
    self.channels.push(Box::new(channel));
    self
  }
}

impl ShaderHashProvider for ScreenChannelDebugger {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .channels
      .iter()
      .for_each(|c| (**c).type_id().hash(hasher))
  }
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for ScreenChannelDebugger {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let frame_coord_x = builder.query::<FragmentPosition>().x();

      let output = val(Vec4::new(0., 0., 0., 1.)).make_local_var();

      let width = builder.query::<RenderBufferSize>().x();

      let step = width / val(self.channels.len() as f32);
      let start = val(0.).make_local_var();
      for channel in &self.channels {
        let start_current = start.load();
        let start_end = start_current + step;
        if_by(
          start_current
            .less_than(frame_coord_x)
            .and(frame_coord_x.less_equal_than(start_end)),
          || {
            output.store(output.load() + channel.to_screen(builder));
          },
        );
        start.store(start_end);
      }

      builder.register::<DefaultDisplay>(output.load());
    })
  }
}

impl ShaderPassBuilder for ScreenChannelDebugger {}

impl ChannelVisualize for FragmentRenderNormal {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>> {
    let normal = builder
      .try_query::<Self>()
      .unwrap_or_else(|| val(Vec3::zero()));

    (normal * val(0.5) + val(Vec3::splat(0.5)), val(1.)).into()
  }
}

impl ChannelVisualize for FragmentUv {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>> {
    let uv = builder
      .try_query::<Self>()
      .unwrap_or_else(|| val(Vec2::zero()));

    (uv, val(0.), val(1.)).into()
  }
}

impl ChannelVisualize for ColorChannel {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>> {
    let value = builder
      .try_query::<Self>()
      .unwrap_or_else(|| val(Vec3::zero()));

    (value, val(1.)).into()
  }
}

impl ChannelVisualize for RoughnessChannel {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>> {
    let value = builder.try_query::<Self>().unwrap_or_else(|| val(0.));
    let value: Node<Vec3<f32>> = value.splat();

    (value, val(1.)).into()
  }
}

impl ChannelVisualize for MetallicChannel {
  fn to_screen(&self, builder: &mut ShaderFragmentBuilderView) -> Node<Vec4<f32>> {
    let value = builder.try_query::<Self>().unwrap_or_else(|| val(0.));
    let value: Node<Vec3<f32>> = value.splat();

    (value, val(1.)).into()
  }
}
