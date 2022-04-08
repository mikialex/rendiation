use crate::{AttachmentReadView, SceneRenderPass, SceneRenderable};

use rendiation_algebra::*;
use rendiation_webgpu::*;

pub struct HighLighter {
  pub data: UniformBufferDataView<HighLightData>,
}

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct HighLightData {
  pub color: Vec4<f32>,
  pub width: f32,
}

impl Default for HighLightData {
  fn default() -> Self {
    Self {
      color: (0., 0.4, 8., 1.).into(),
      width: 2.,
      ..Default::default()
    }
  }
}

impl HighLighter {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      data: UniformBufferDataResource::create_with_source(Default::default(), &gpu.device)
        .create_view(Default::default()),
    }
  }
}

impl HighLighter {
  pub fn draw<T: 'static>(&self, mask: AttachmentReadView<T>) -> impl PassContent + '_ {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
    .draw_quad()
  }
}

pub struct HighLightComposeTask<'a, T> {
  mask: AttachmentReadView<T>,
  lighter: &'a HighLighter,
}

impl<'a, T> ShaderPassBuilder for HighLightComposeTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lighter.data, SB::Material);
    ctx.binding.bind(&self.mask, SB::Material);
  }
}

impl<'a, T> ShaderGraphProvider for HighLightComposeTask<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding
        .uniform_by(&self.lighter.data, SB::Material)
        .expand();

      let mask = binding.uniform_by(&self.mask, SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      builder.set_fragment_out(
        0,
        (uniform.color.xyz(), edge_intensity(uv) * uniform.color.w()).into(),
      )?;
      todo!()
    })
  }
}

wgsl_function!(
  fn edge_intensity(uv: vec2<f32>) -> f32 {
    var x_step: f32 = pass_info.texel_size.x * highlighter.width;
    var y_step: f32 = pass_info.texel_size.y * highlighter.width;

    var all: f32 = 0.0;
    all = all + textureSample(mask, tex_sampler, in.uv).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y)).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x, in.uv.y + y_step)).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y+ y_step)).x;

    var intensity = (1.0 - 2.0 * abs(all / 4. - 0.5)) * highlighter.color.a;
  }
);

pub struct HighLightDrawMaskTask<T> {
  objects: T,
}

pub fn highlight<T>(objects: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { objects }
}

struct HighLightMaskDispatcher;

impl ShaderHashProvider for HighLightMaskDispatcher {}
impl ShaderPassBuilder for HighLightMaskDispatcher {}

impl ShaderGraphProvider for HighLightMaskDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| builder.set_fragment_out(0, Vec4::one().into()))
  }
}

impl<'i, T> PassContentWithCamera for HighLightDrawMaskTask<T>
where
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    for model in self.objects {
      model.render(pass, &HighLightMaskDispatcher, camera)
    }
  }
}
