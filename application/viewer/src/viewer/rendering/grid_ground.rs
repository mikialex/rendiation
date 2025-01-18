use rendiation_infinity_primitive::*;

use crate::*;

pub struct GridGround<'a> {
  pub shading: &'a UniformBufferCachedDataView<GridEffect>,
  pub plane: &'a UniformBufferCachedDataView<ShaderPlane>,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for GridGround<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass);

    let effect = InfinityShaderPlaneEffect {
      plane: self.plane,
      camera: self.camera,
    };

    let grid = GridGroundShading {
      shading: self.shading,
    };

    let com: [&dyn RenderComponent; 3] = [&base, &effect, &grid];
    let com = RenderArray(com);

    com.render(&mut pass.ctx, PLANE_DRAW_CMD)
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct GridEffect {
  pub scale: Vec2<f32>,
  pub color: Vec4<f32>,
}

impl Default for GridEffect {
  fn default() -> Self {
    Self {
      scale: Vec2::one(),
      color: Vec4::splat(1.),
      ..Zeroable::zeroed()
    }
  }
}

pub struct GridGroundShading<'a> {
  pub shading: &'a UniformBufferCachedDataView<GridEffect>,
}
impl ShaderHashProvider for GridGroundShading<'_> {
  shader_hash_type_id! {GridGroundShading<'static>}
}
impl ShaderPassBuilder for GridGroundShading<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.shading);
  }
}
impl GraphicsShaderProvider for GridGroundShading<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let shading = binding.bind_by(&self.shading).load();
      let world_position = builder.query::<FragmentWorldPosition>();

      let grid = grid(world_position, shading);

      builder.register::<DefaultDisplay>(grid);
    })
  }
}

#[shader_fn]
fn grid(position: Node<Vec3<f32>>, config: Node<GridEffect>) -> Node<Vec4<f32>> {
  let coord = position.xz() * GridEffect::scale(config);
  let grid =
    ((coord - val(Vec2::splat(0.5))).fract() - val(Vec2::splat(0.5))).abs() / coord.fwidth();
  let lined = grid.x().min(grid.y());
  (val(0.2), val(0.2), val(0.2), val(1.1) - lined.min(val(1.0))).into()
}
