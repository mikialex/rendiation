use rendiation_algebra::Vec2;
use rendiation_webgpu::{PassTargetFormatInfo, PipelineBuilder};
use shadergraph::*;

use crate::MaterialStates;

pub fn full_screen_vertex_shader(
  builder: &mut PipelineBuilder,
  blend: Option<wgpu::BlendState>,
  format_info: &PassTargetFormatInfo,
) {
  builder.primitive_state = wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::TriangleStrip,
    front_face: wgpu::FrontFace::Cw,
    ..Default::default()
  };

  MaterialStates {
    blend,
    depth_write_enabled: false,
    depth_compare: wgpu::CompareFunction::Always,
    ..Default::default()
  }
  .apply_pipeline_builder(builder, format_info);

  builder
    .declare_io_struct(
      "
        struct VertexOutput {
          [[builtin(position)]] position: vec4<f32>;
          [[location(0)]] uv: vec2<f32>;
        };
      ",
    )
    .include_vertex_entry(
      "
      [[stage(vertex)]]
      fn vs_main_full_screen(
        [[builtin(vertex_index)]] vertex_index: u32,
      ) -> VertexOutput {
        var out: VertexOutput;

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

        return out;
      }
    ",
    )
    .use_vertex_entry("vs_main_full_screen");
}

pub fn build_shader_graph() {
  // let left = -1.;
  // let right = 1.;
  // let top = 1;
  // let bottom = -1.0;
  // let depth = 0.0;

  // let vertex_index = query_built_in::<VertexIndex>();
  // switch_by(
  //   vertex_index,
  //   [(0, ((left, bottom, depth, 1.).into()), (0., 1.).into())],
  //   ((right, bottom, depth, 1.).into(), (1., 1.).into()),
  // )
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod, PartialEq, ShaderUniform)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}
