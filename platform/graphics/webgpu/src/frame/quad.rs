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

  let position = val(Vec4::<f32>::default()).make_local_var();
  let uv = val(Vec2::<f32>::default()).make_local_var();

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
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for FullScreenQuad {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::TriangleStrip,
        front_face: FrontFace::Cw,
        ..Default::default()
      };
      let out = generate_quad(builder.query::<VertexIndex>(), 0.).expand();
      builder.register::<ClipPosition>(out.position);
      builder.set_vertex_out::<FragmentUv>(out.uv);
    });

    builder.fragment(|builder, _| {
      builder.frag_output.iter_mut().for_each(|p| {
        if p.is_blendable() && self.blend.is_some() {
          p.states.blend = self.blend;
        }
      });

      if let Some(depth) = &mut builder.depth_stencil {
        depth.depth_compare = CompareFunction::Always;
        depth.depth_write_enabled = false;
      }
    })
  }
}

pub struct QuadDraw<T> {
  pub quad: FullScreenQuad,
  pub content: T,
  pub viewport: Option<Vec4<f32>>,
}

impl<T> QuadDraw<T> {
  pub fn with_viewport(mut self, viewport: Vec4<f32>) -> Self {
    self.viewport = Some(viewport);
    self
  }
}

pub trait UseQuadDraw: Sized {
  // default to not override blending config
  fn draw_quad(self) -> QuadDraw<Self> {
    self.draw_quad_with_blend(None)
  }

  fn draw_quad_with_alpha_blending(self) -> QuadDraw<Self> {
    QuadDraw {
      content: self,
      quad: Default::default(),
      viewport: None,
    }
  }

  fn draw_quad_with_blend(self, blend: Option<BlendState>) -> QuadDraw<Self> {
    QuadDraw {
      content: self,
      quad: FullScreenQuad { blend },
      viewport: None,
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
  T: RenderComponent,
{
  fn render(&mut self, pass: &mut FrameRenderPass) {
    if let Some(viewport) = self.viewport {
      let [x, y, w, h] = viewport.into();
      pass.set_viewport(x, y, w, h, 0., 1.);
    }

    let mut base = default_dispatcher(pass, false);
    base.auto_write = false;
    let components: [&dyn RenderComponent; 3] = [&base, &self.quad, &self.content];
    RenderArray(components).render(&mut pass.ctx, QUAD_DRAW_CMD);

    if self.viewport.is_some() {
      let (w, h) = pass.size().into_f32();
      pass.set_viewport(0., 0., w, h, 0., 1.);
    }
  }
}
