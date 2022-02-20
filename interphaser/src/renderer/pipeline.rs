use shadergraph::*;
use webgpu::*;

use crate::{renderer::UIGlobalParameter, UIVertex};

pub struct SolidUIPipeline {
  pub target_format: webgpu::TextureFormat,
}

impl ShaderGraphProvider for SolidUIPipeline {
  fn build_vertex(
    &self,
    builder: &mut shadergraph::ShaderGraphVertexBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
    builder.primitive_state = webgpu::PrimitiveState {
      topology: webgpu::PrimitiveTopology::TriangleList,
      cull_mode: None,
      ..Default::default()
    };

    let position = builder.query::<GeometryPosition>()?.get();
    let color = builder.query::<GeometryColor>()?.get();

    let global = builder
      .register_uniform::<UniformBuffer<UIGlobalParameter>>(SemanticBinding::Global)
      .expand();

    let vertex: Node<Vec4<_>> = (
      consts(2.0) * position.x() / global.screen_size.x() - consts(1.0),
      consts(1.0) - consts(2.0) * position.y() / global.screen_size.y(),
      consts(0.0),
      consts(1.0),
    )
      .into();

    builder.vertex_position.set(vertex);
    builder.set_vertex_out::<FragmentColor>(color);

    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut shadergraph::ShaderGraphFragmentBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    builder.push_fragment_out_slot(ColorTargetState {
      format: self.target_format,
      blend: Some(webgpu::BlendState::ALPHA_BLENDING),
      write_mask: webgpu::ColorWrites::ALL,
    });

    let color = builder.query::<FragmentColor>()?.get();
    let color = (color, 1.).into();
    builder.set_fragment_out(0, color)?;
    Ok(())
  }
}

pub struct TextureUIPipeline {
  pub target_format: webgpu::TextureFormat,
}

impl ShaderGraphProvider for TextureUIPipeline {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.register_vertex::<UIVertex>(VertexStepMode::Vertex);
    builder.primitive_state = webgpu::PrimitiveState {
      topology: webgpu::PrimitiveTopology::TriangleList,
      cull_mode: None,
      ..Default::default()
    };

    let position = builder.query::<GeometryPosition>()?.get();
    let color = builder.query::<GeometryColor>()?.get();
    let uv = builder.query::<GeometryUV>()?.get();

    let global = builder
      .register_uniform::<UniformBuffer<UIGlobalParameter>>(SemanticBinding::Global)
      .expand();

    let vertex: Node<Vec4<_>> = (
      consts(2.0) * position.x() / global.screen_size.x() - consts(1.0),
      consts(1.0) - consts(2.0) * position.y() / global.screen_size.y(),
      consts(0.0),
      consts(1.0),
    )
      .into();

    builder.vertex_position.set(vertex);
    builder.set_vertex_out::<FragmentColor>(color);
    builder.set_vertex_out::<FragmentUv>(uv);

    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.push_fragment_out_slot(ColorTargetState {
      format: self.target_format,
      blend: Some(webgpu::BlendState::ALPHA_BLENDING),
      write_mask: webgpu::ColorWrites::ALL,
    });
    use webgpu::container::*;
    let texture = builder.register_uniform::<SemanticGPUTexture2d<Self>>(SemanticBinding::Material);
    let sampler = builder.register_uniform::<SemanticGPUSampler<Self>>(SemanticBinding::Material);
    let uv = builder.query::<FragmentUv>()?.get();
    let color = texture.sample(sampler, uv);
    builder.set_fragment_out(0, color)?;
    Ok(())
  }
}
