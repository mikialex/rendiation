use crate::*;

#[shader_struct]
#[derive(Copy, Clone, Default)]
pub struct FullScreenVertexOut {
  pub position: Vec4<f32>,
  pub uv: Vec2<f32>,
}

/// generate the full screen triangle by vertex index 0..3, see [FULL_SCREEN_TRIANGLE_DRAW_CMD].
///
/// the single triangle covers the whole clip space and is clipped to the viewport. compare to the
/// two triangles quad, it avoids the redundant fragment shading of the 2x2 pixel quads along the
/// diagonal edge. the uv in the visible area is still in 0..1.
pub fn generate_full_screen_triangle(
  vertex_index: Node<u32>,
  depth: f32,
) -> Node<FullScreenVertexOut> {
  // vertex index 0, 1, 2 maps to uv (0, 0), (2, 0), (0, 2)
  let u = ((vertex_index << val(1)) & val(2)).into_f32();
  let v = (vertex_index & val(2)).into_f32();

  // uv y axis is flipped compare to the ndc
  let x = u * val(2.) - val(1.);
  let y = val(1.) - v * val(2.);

  ENode::<FullScreenVertexOut> {
    position: (x, y, val(depth), val(1.)).into(),
    uv: (u, v).into(),
  }
  .construct()
}

pub struct FullScreenTriangle {
  blend: Option<BlendState>,
}

impl Default for FullScreenTriangle {
  fn default() -> Self {
    Self {
      blend: Some(BlendState::ALPHA_BLENDING),
    }
  }
}

impl ShaderPassBuilder for FullScreenTriangle {}
impl ShaderHashProvider for FullScreenTriangle {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.blend);
  }
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for FullScreenTriangle {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      *builder.primitive_state() = PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        front_face: FrontFace::Cw,
        ..Default::default()
      };
      let out = generate_full_screen_triangle(builder.query::<VertexIndex>(), 0.).expand();
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
        depth.depth_compare = Some(CompareFunction::Always);
        depth.depth_write_enabled = Some(false);
      }
    })
  }
}

pub struct FullScreenDraw<T> {
  pub triangle: FullScreenTriangle,
  pub content: T,
  pub viewport: Option<Vec4<f32>>,
}

impl<T> FullScreenDraw<T> {
  pub fn with_viewport(mut self, viewport: Vec4<f32>) -> Self {
    self.viewport = Some(viewport);
    self
  }
}

pub trait UseFullScreenDraw: Sized {
  // default to not override blending config
  fn draw_full_screen(self) -> FullScreenDraw<Self> {
    self.draw_full_screen_with_blend(None)
  }

  fn draw_full_screen_with_alpha_blending(self) -> FullScreenDraw<Self> {
    FullScreenDraw {
      content: self,
      triangle: Default::default(),
      viewport: None,
    }
  }

  fn draw_full_screen_with_blend(self, blend: Option<BlendState>) -> FullScreenDraw<Self> {
    FullScreenDraw {
      content: self,
      triangle: FullScreenTriangle { blend },
      viewport: None,
    }
  }
}

pub const FULL_SCREEN_TRIANGLE_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..3,
  instances: 0..1,
};

impl<T> UseFullScreenDraw for T {}

impl<T> PassContent for FullScreenDraw<T>
where
  T: RenderComponent,
{
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let previous_viewport = pass.viewport();
    if let Some(viewport) = self.viewport {
      let [x, y, w, h] = viewport.into();
      pass.set_viewport(x, y, w, h, 0., 1.);
    }

    let mut base = default_dispatcher(pass, false);
    base.auto_write = false;
    let components: [&dyn RenderComponent; 3] = [&base, &self.triangle, &self.content];
    RenderArray(components).render(
      &mut pass.ctx,
      RenderMethod::TraditionalDraw(FULL_SCREEN_TRIANGLE_DRAW_CMD),
    );

    if self.viewport.is_some() {
      pass.restore_viewport(previous_viewport);
    }
  }
}
