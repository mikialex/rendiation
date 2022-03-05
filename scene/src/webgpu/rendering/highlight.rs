use std::{any::TypeId, hash::Hash, rc::Rc};

use crate::{
  AttachmentOwnedReadView, PassContent, RenderPassGPUInfoData, Scene, SceneRenderPass,
  SceneRenderable,
};

use rendiation_algebra::*;
use rendiation_texture::TextureSampler;
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
      data: UniformBufferData::create(&gpu.device, Default::default()),
    }
  }
}

impl HighLighter {
  pub fn draw(&self, mask: AttachmentOwnedReadView) -> HighLightComposeTask {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
  }
}

pub struct HighLightComposeTask<'a> {
  mask: AttachmentOwnedReadView,
  lighter: &'a HighLighter,
}

impl<'a> ShaderPassBuilder for HighLightComposeTask<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.setup_uniform(&self.lighter.data, SB::Material)
  }
}

impl<'a> ShaderGraphProvider for HighLightComposeTask<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding
        .uniform_by(&self.lighter.data, SB::Material)
        .expand();

      let uv = builder.query::<FragmentUv>()?;
      builder.set_fragment_out(0, (uniform.color, edge_intensity(uv)))
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

pub struct HighLightDrawMaskTask<'a, T> {
  objects: T,
  scene: &'a mut Scene,
}

pub fn highlight<T>(objects: T, scene: &mut Scene) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { objects, scene }
}

struct HighLightMaskDispatcher;

impl ShaderGraphProvider for HighLightMaskDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      builder.set_fragment_out(0, Vec4::one().into());
      Ok(())
    })
  }
}

impl<'s, 'i, T> PassContent for HighLightDrawMaskTask<'s, T>
where
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  fn render(&mut self, gpu: &GPU, pass: &mut GPURenderPass) {
    for model in self.objects {
      model.setup_pass(
        gpu,
        pass,
        self.scene.active_camera.as_ref().unwrap(),
        &self.scene.resources,
      )
    }
  }
}
