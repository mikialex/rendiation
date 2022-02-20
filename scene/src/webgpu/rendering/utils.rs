use rendiation_algebra::Vec2;
use shadergraph::*;

use crate::MaterialStates;

static BuildVertex: &str = "
fn generate_quad(
  vertex_index: u32
) -> ? {
  var left: f32 = -1.0;
  var right: f32 = 1.0;
  var top: f32 = 1.0;
  var bottom: f32 = -1.0;
  var depth: f32 = 0.0;

  switch (i32(vertex_index)) {
    case 0: {
      out.position = vec4<f32>(left, top, depth, 1.);
      out.uv = vec2<f32>(0., 0.);
    }
    case 1: {
      out.position = vec4<f32>(right, top, depth, 1.);
      out.uv = vec2<f32>(1., 0.);
    }
    case 2: {
      out.position = vec4<f32>(left, bottom, depth, 1.);
      out.uv = vec2<f32>(0., 1.);
    }
    default: {
      out.position = vec4<f32>(right, bottom, depth, 1.);
      out.uv = vec2<f32>(1., 1.);
    }
  }
}
";

struct FullScreenQuad {
  blend: Option<wgpu::BlendState>,
}

impl ShaderGraphProvider for FullScreenQuad {
  fn build_vertex(
    &self,
    _builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.primitive_state = wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleStrip,
      front_face: wgpu::FrontFace::Cw,
      ..Default::default()
    };
    todo!();
    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    MaterialStates {
      blend: self.blend,
      depth_write_enabled: false,
      depth_compare: wgpu::CompareFunction::Always,
      ..Default::default()
    }
    .apply_pipeline_builder(builder);
    Ok(())
  }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod, PartialEq, ShaderUniform)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}
