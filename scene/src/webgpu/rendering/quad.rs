use rendiation_algebra::Vec2;
use rendiation_webgpu::{ShaderHashProvider, ShaderPassBuilder};
use shadergraph::*;

use crate::*;

#[derive(Copy, Clone, ShaderStruct)]
pub struct QuadVertexOut {
  pub position: Vec4<f32>,
  pub uv: Vec2<f32>,
}

wgsl_function!(
  fn generate_quad(
    vertex_index: u32
  ) -> QuadVertexOut {
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
);

struct FullScreenQuad {
  blend: Option<wgpu::BlendState>,
}

impl Default for FullScreenQuad {
  fn default() -> Self {
    Self { blend: None }
  }
}

impl ShaderPassBuilder for FullScreenQuad {
  fn setup_pass(&self, ctx: &mut rendiation_webgpu::GPURenderPassCtx) {
    ctx.pass.draw(0..4, 0..1)
  }
}
impl ShaderHashProvider for FullScreenQuad {}
impl ShaderGraphProvider for FullScreenQuad {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.primitive_state = wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        front_face: wgpu::FrontFace::Cw,
        ..Default::default()
      };
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      MaterialStates {
        blend: self.blend,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        ..Default::default()
      }
      .apply_pipeline_builder(builder);
      Ok(())
    })
  }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod, PartialEq, ShaderStruct)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}

pub struct QuadDraw<T> {
  quad: FullScreenQuad,
  content: T,
}

pub trait UseQuadDraw: Sized {
  fn draw_quad(self) -> QuadDraw<Self> {
    QuadDraw {
      content: self,
      quad: Default::default(),
    }
  }
}

impl<T> UseQuadDraw for T {}

impl<T> PassContent for QuadDraw<T>
where
// T: RenderComponentAny,
{
  fn render(&mut self, pass: &mut SceneRenderPass) {
    // let components: [&dyn RenderComponentAny; 2] = [&self.quad, &self.content];
    // RenderEmitter::new(components.as_slice()).render(&mut pass.ctx);
  }
}
