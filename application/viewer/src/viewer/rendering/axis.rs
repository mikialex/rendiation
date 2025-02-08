use rendiation_infinity_primitive::*;

use crate::*;

pub struct CoordinateInfinityAxis<'a> {
  pub shading: &'a UniformBufferCachedDataView<Vec4<f32>>,
  pub line: &'a UniformBufferCachedDataView<ShaderLine>,
  pub reversed_depth: bool,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for CoordinateInfinityAxis<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth);

    let line = InfinityShaderLineEffect {
      line: self.line,
      camera: self.camera,
    };

    let shading = LineShading {
      shading: self.shading,
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

impl ShaderPassBuilder for LineShading<'_> {}

impl GraphicsShaderProvider for LineShading<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      builder.register::<DefaultDisplay>(val(Vec4::one()));
    })
  }
}
