use shadergraph::*;
use webgpu::*;

use crate::{renderer::UIGlobalParameter, UIVertex};

pub struct SolidUIPipeline {
  pub target_format: webgpu::TextureFormat,
}

impl ShaderGraphProvider for SolidUIPipeline {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    let global = builder.uniform::<UniformBufferView<UIGlobalParameter>>(SemanticBinding::Global);

    builder.vertex(|builder, _| {
      builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
      builder.primitive_state = webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleList,
        cull_mode: None,
        ..Default::default()
      };

      let position = builder.query::<GeometryPosition>()?.get();
      let color = builder.query::<GeometryColorWithAlpha>()?.get();

      let global = global.using().expand();

      let vertex = (
        consts(2.0) * position.x() / global.screen_size.x() - consts(1.0),
        consts(1.0) - consts(2.0) * position.y() / global.screen_size.y(),
        consts(0.0),
        consts(1.0),
      );

      builder.register::<ClipPosition>(vertex);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color);

      Ok(())
    })?;

    builder.fragment(|builder, _| {
      builder.out_by(channel(self.target_format).with_alpha_blend());

      let color = builder.query::<FragmentColorAndAlpha>()?.get();
      builder.set_fragment_out(0, color)
    })
  }
}

pub struct TextureUIPipeline {
  pub target_format: webgpu::TextureFormat,
}

impl ShaderGraphProvider for TextureUIPipeline {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    let global = builder.uniform::<UniformBufferView<UIGlobalParameter>>(SemanticBinding::Global);

    builder.vertex(|builder, _| {
      builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
      builder.primitive_state = webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleList,
        cull_mode: None,
        ..Default::default()
      };

      let position = builder.query::<GeometryPosition>()?.get();
      let color = builder.query::<GeometryColorWithAlpha>()?.get();
      let uv = builder.query::<GeometryUV>()?.get();

      let global = global.using().expand();

      let vertex: Node<Vec4<_>> = (
        consts(2.0) * position.x() / global.screen_size.x() - consts(1.0),
        consts(1.0) - consts(2.0) * position.y() / global.screen_size.y(),
        consts(0.0),
        consts(1.0),
      )
        .into();

      builder.register::<ClipPosition>(vertex);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color);
      builder.set_vertex_out::<FragmentUv>(uv);

      Ok(())
    })?;

    use webgpu::container::*;

    builder.fragment(|builder, binding| {
      builder.out_by(channel(self.target_format).with_alpha_blend());
      let uv = builder.query::<FragmentUv>()?.get();
      let texture = binding.uniform::<GPUTexture2dView>(SemanticBinding::Material);
      let sampler = binding.uniform::<GPUSamplerView>(SemanticBinding::Material);

      builder.set_fragment_out(0, texture.sample(sampler, uv))
    })
  }
}
