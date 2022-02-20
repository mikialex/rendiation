use shadergraph::*;
use webgpu::UniformBuffer;

use crate::{renderer::UIGlobalParameter, UIVertex};

pub struct SolidUIPipeline {
  target_format: webgpu::TextureFormat,
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

    let vertex = (
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
    builder.set_fragment_out(0, color);
    Ok(())
  }
}

pub struct TextureBindGroup {
  pub bindgroup: webgpu::BindGroup,
}

impl TextureBindGroup {
  pub fn new(
    device: &webgpu::Device,
    layout: &webgpu::BindGroupLayout,
    sampler: &webgpu::Sampler,
    view: &webgpu::TextureView,
  ) -> Self {
    let bindgroup = device.create_bind_group(&webgpu::BindGroupDescriptor {
      layout,
      entries: &[
        webgpu::BindGroupEntry {
          binding: 0,
          resource: webgpu::BindingResource::TextureView(view),
        },
        webgpu::BindGroupEntry {
          binding: 1,
          resource: webgpu::BindingResource::Sampler(sampler),
        },
      ],
      label: None,
    });
    Self { bindgroup }
  }
}

impl TextureBindGroup {
  fn get_shader_header() -> &'static str {
    "
    [[group(1), binding(0)]]
    var r_color: texture_2d<f32>;

    [[group(1), binding(1)]]
    var r_sampler: sampler;
    "
  }

  pub fn create_bind_group_layout(device: &webgpu::Device) -> webgpu::BindGroupLayout {
    device.create_bind_group_layout(&webgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        webgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Texture {
            multisampled: false,
            sample_type: webgpu::TextureSampleType::Float { filterable: true },
            view_dimension: webgpu::TextureViewDimension::D2,
          },
          count: None,
        },
        webgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: webgpu::ShaderStages::FRAGMENT,
          ty: webgpu::BindingType::Sampler(webgpu::SamplerBindingType::Filtering),
          count: None,
        },
      ],
    })
  }
}

pub struct TextureUIPipeline {
  target_format: webgpu::TextureFormat,
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

    let vertex = (
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

    let texture = builder.register_uniform()?.get();
    let sampler = builder.register_uniform()?.get();
    let uv = builder.query::<FragmentUv>();
    let color = texture.sample(sampler, uv);
    builder.set_fragment_out(0, color);
    Ok(())
  }
}
