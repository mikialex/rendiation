use crate::*;

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct QuadVertexOut {
  pub position: Vec4<f32>,
  pub uv: Vec2<f32>,
}

// fn generate_quad2(vertex_index: Node<u32>) -> Node<QuadVertexOut> {
//   let left = val(-1.0);
//   let right = val(1.0);
//   let top = val(1.0);
//   let bottom = val(-1.0);
//   let depth = val(0.0);

//   // let quad = QuadVertexOut::default().mutable();

//   switch_by(vertex_index).case(0, || {
//     quad.set()
//   }).end()
// }

wgsl_fn!(
  fn generate_quad(
    vertex_index: u32
  ) -> QuadVertexOut {
    var left: f32 = -1.0;
    var right: f32 = 1.0;
    var top: f32 = 1.0;
    var bottom: f32 = -1.0;
    var depth: f32 = 0.0;

    var out: QuadVertexOut;

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
);

pub struct FullScreenQuad {
  blend: Option<webgpu::BlendState>,
}

impl Default for FullScreenQuad {
  fn default() -> Self {
    Self {
      blend: Some(webgpu::BlendState::ALPHA_BLENDING),
    }
  }
}

impl ShaderPassBuilder for FullScreenQuad {}
impl ShaderHashProvider for FullScreenQuad {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.blend.hash(hasher)
  }
}
impl GraphicsShaderProvider for FullScreenQuad {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.primitive_state = webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleStrip,
        front_face: webgpu::FrontFace::Cw,
        ..Default::default()
      };
      let out = generate_quad(builder.query::<VertexIndex>()?).expand();
      builder.register::<ClipPosition>(out.position);
      builder.set_vertex_out::<FragmentUv>(out.uv);
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      MaterialStates {
        blend: self.blend,
        depth_write_enabled: false,
        depth_compare: webgpu::CompareFunction::Always,
        ..Default::default()
      }
      .apply_pipeline_builder(builder);
      Ok(())
    })
  }
}

pub struct QuadDraw<T> {
  pub quad: FullScreenQuad,
  pub content: T,
}

pub trait UseQuadDraw: Sized {
  /// default use alpha blend
  fn draw_quad(self) -> QuadDraw<Self> {
    QuadDraw {
      content: self,
      quad: Default::default(),
    }
  }
  fn draw_quad_with_blend(self, blend: Option<BlendState>) -> QuadDraw<Self> {
    QuadDraw {
      content: self,
      quad: FullScreenQuad { blend },
    }
  }
}

pub const QUAD_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..4,
  instances: 0..1,
};

impl<T> UseQuadDraw for T {}

impl<T> PassContent for QuadDraw<T>
where
  T: RenderComponentAny,
{
  default fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut base = default_dispatcher(pass);
    base.auto_write = false;
    let components: [&dyn RenderComponentAny; 3] = [&base, &self.quad, &self.content];

    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, QUAD_DRAW_CMD);
  }
}
