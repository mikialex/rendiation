use crate::*;

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct QuadVertexOut {
  pub position: Vec4<f32>,
  pub uv: Vec2<f32>,
}

pub fn generate_quad(vertex_index: Node<u32>, depth: f32) -> Node<QuadVertexOut> {
  let left = -1.0;
  let right = 1.0;
  let top = 1.0;
  let bottom = -1.0;

  let position = val(Vec4::default()).make_local_var();
  let uv = val(Vec2::default()).make_local_var();

  switch_by(vertex_index)
    .case(0, || {
      position.store(Vec4::new(left, top, depth, 1.));
      uv.store(Vec2::new(0., 0.));
    })
    .case(1, || {
      position.store(Vec4::new(right, top, depth, 1.));
      uv.store(Vec2::new(1., 0.));
    })
    .case(2, || {
      position.store(Vec4::new(left, bottom, depth, 1.));
      uv.store(Vec2::new(0., 1.));
    })
    .end_with_default(|| {
      position.store(Vec4::new(right, bottom, depth, 1.));
      uv.store(Vec2::new(1., 1.));
    });

  ENode::<QuadVertexOut> {
    position: position.load(),
    uv: uv.load(),
  }
  .construct()
}

pub struct FullScreenQuad {
  blend: Option<BlendState>,
}

impl Default for FullScreenQuad {
  fn default() -> Self {
    Self {
      blend: Some(BlendState::ALPHA_BLENDING),
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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::TriangleStrip,
        front_face: FrontFace::Cw,
        ..Default::default()
      };
      let out = generate_quad(builder.query::<VertexIndex>()?, 0.).expand();
      builder.register::<ClipPosition>(out.position);
      builder.set_vertex_out::<FragmentUv>(out.uv);
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      builder.frag_output.iter_mut().for_each(|(_, state)| {
        state.blend = self.blend;
      });

      if let Some(depth) = &mut builder.depth_stencil {
        depth.depth_compare = CompareFunction::Always;
        depth.depth_write_enabled = false;
      }

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
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut base = default_dispatcher(pass);
    base.auto_write = false;
    let components: [&dyn RenderComponentAny; 3] = [&base, &self.quad, &self.content];

    RenderSlice::new(components.as_slice()).render(&mut pass.ctx, QUAD_DRAW_CMD);
  }
}
