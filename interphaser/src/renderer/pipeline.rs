use rendiation_shader_api::*;

use crate::*;

pub struct SolidUIPipeline {
  pub target_format: TextureFormat,
}

impl GraphicsShaderProvider for SolidUIPipeline {
  fn build(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> Result<(), rendiation_shader_api::ShaderBuildError> {
    builder.set_binding_slot(0);
    let global = builder.binding::<UniformBufferDataView<UIGlobalParameter>>();

    builder.vertex(|builder, _| {
      builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        cull_mode: None,
        ..Default::default()
      };

      let position = builder.query::<GeometryPosition2D>()?;
      let color = builder.query::<GeometryColorWithAlpha>()?;

      let global = global.using().load().expand();

      let vertex = (
        val(2.0) * position.x() / global.screen_size.x() - val(1.0),
        val(1.0) - val(2.0) * position.y() / global.screen_size.y(),
        val(0.0),
        val(1.0),
      );

      builder.register::<ClipPosition>(vertex);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color);

      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let color = builder.query::<FragmentColorAndAlpha>()?;

      let slot = builder.define_out_by(channel(self.target_format).with_alpha_blend());
      builder.store_fragment_out(slot, color)
    })
  }
}

pub struct TextureUIPipeline {
  pub target_format: TextureFormat,
}

impl GraphicsShaderProvider for TextureUIPipeline {
  fn build(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> Result<(), rendiation_shader_api::ShaderBuildError> {
    builder.set_binding_slot(0);
    let global = builder.binding::<UniformBufferDataView<UIGlobalParameter>>();

    builder.vertex(|builder, _| {
      builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        cull_mode: None,
        ..Default::default()
      };

      let position = builder.query::<GeometryPosition2D>()?;
      let color = builder.query::<GeometryColorWithAlpha>()?;
      let uv = builder.query::<GeometryUV>()?;

      let global = global.using().load().expand();

      let vertex = vec4_node((
        val(2.0) * position.x() / global.screen_size.x() - val(1.0),
        val(1.0) - val(2.0) * position.y() / global.screen_size.y(),
        val(0.0),
        val(1.0),
      ));

      builder.register::<ClipPosition>(vertex);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color);
      builder.set_vertex_out::<FragmentUv>(uv);

      Ok(())
    })?;

    builder.set_binding_slot(1);

    builder.fragment(|builder, binding| {
      builder.define_out_by(channel(self.target_format).with_alpha_blend());
      let uv = builder.query::<FragmentUv>()?;
      let texture = binding.binding::<GPU2DTextureView>();
      let sampler = binding.binding::<GPUSamplerView>();

      builder.store_fragment_out(0, texture.sample(sampler, uv))
    })
  }
}
