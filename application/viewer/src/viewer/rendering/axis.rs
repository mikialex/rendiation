use rendiation_infinity_primitive::*;

use crate::*;

pub struct WorldCoordinateAxis {
  x: AxisData,
  y: AxisData,
  z: AxisData,
}

impl WorldCoordinateAxis {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      x: AxisData::new(
        gpu,
        Vec4::new(1., 0., 0., 1.),
        Ray3::new(
          Vec3::new(0., 0., 0.),
          Vec3::new(1., 0., 0.).into_normalized(),
        ),
      ),
      y: AxisData::new(
        gpu,
        Vec4::new(0., 1., 0., 1.),
        Ray3::new(
          Vec3::new(0., 0., 0.),
          Vec3::new(0., 1., 0.).into_normalized(),
        ),
      ),
      z: AxisData::new(
        gpu,
        Vec4::new(0., 0., 1., 1.),
        Ray3::new(
          Vec3::new(0., 0., 0.),
          Vec3::new(0., 0., 1.).into_normalized(),
        ),
      ),
    }
  }
}

pub struct DrawWorldAxis<'a> {
  pub data: &'a WorldCoordinateAxis,
  pub reversed_depth: bool,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for DrawWorldAxis<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    DrawAxis {
      data: &self.data.x,
      reversed_depth: self.reversed_depth,
      camera: self.camera,
    }
    .render(pass);
    DrawAxis {
      data: &self.data.y,
      reversed_depth: self.reversed_depth,
      camera: self.camera,
    }
    .render(pass);
    DrawAxis {
      data: &self.data.z,
      reversed_depth: self.reversed_depth,
      camera: self.camera,
    }
    .render(pass);
  }
}

pub struct AxisData {
  pub shading: UniformBufferCachedDataView<Vec4<f32>>,
  pub line: UniformBufferCachedDataView<ShaderLine>,
}

impl AxisData {
  pub fn new(gpu: &GPU, color: Vec4<f32>, ray: Ray3) -> Self {
    Self {
      line: UniformBufferCachedDataView::create(
        &gpu.device,
        ShaderLine::new(ray.origin, ray.direction.value),
      ),
      shading: UniformBufferCachedDataView::create(&gpu.device, color),
    }
  }
}

pub struct DrawAxis<'a> {
  pub data: &'a AxisData,
  pub reversed_depth: bool,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for DrawAxis<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth);

    let line = InfinityShaderLineEffect {
      line: &self.data.line,
      camera: self.camera,
      reversed_depth: self.reversed_depth,
    };

    let shading = LineShading {
      shading: &self.data.shading,
    };

    let com: [&dyn RenderComponent; 3] = [&base, &line, &shading];
    let com = RenderArray(com);

    com.render(&mut pass.ctx, LINE_DRAW_CMD)
  }
}

struct LineShading<'a> {
  shading: &'a UniformBufferCachedDataView<Vec4<f32>>,
}

impl ShaderHashProvider for LineShading<'_> {
  shader_hash_type_id! {LineShading<'static>}
}

impl ShaderPassBuilder for LineShading<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.shading);
  }
}

impl GraphicsShaderProvider for LineShading<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let color = binding.bind_by(self.shading).load();
      builder.register::<DefaultDisplay>(color);
    })
  }
}
