use std::{any::TypeId, hash::Hash, rc::Rc};

use crate::{
  AttachmentOwnedReadView, PassContent, RenderPassGPUInfoData, Scene, SceneRenderPass,
  SceneRenderable,
};

use rendiation_algebra::*;
use rendiation_texture::TextureSampler;
use rendiation_webgpu::*;

pub struct HighLighter {
  pub data: UniformBufferData<HighLightData>,
}

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, ShaderStruct)]
pub struct HighLightData {
  pub color: Vec4<f32>,
  pub width: f32,
}

impl Default for HighLightData {
  fn default() -> Self {
    Self {
      color: (0., 0.4, 8., 1.).into(),
      width: 2.,
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
  pub fn draw(&self, mask: AttachmentOwnedReadView<wgpu::TextureFormat>) -> HighLightComposeTask {
    HighLightComposeTask {
      mask,
      lighter: self,
      bindgroup: None,
      pipeline: None,
    }
  }
}

pub struct HighLightComposeTask<'a> {
  mask: AttachmentOwnedReadView<wgpu::TextureFormat>,
  lighter: &'a HighLighter,
  bindgroup: Option<wgpu::BindGroup>,
  pipeline: Option<Rc<wgpu::RenderPipeline>>,
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    let resources = &mut scene.resources.content;
    self.lighter.data.update(&gpu.queue);

    let pass_info = ctx.pass_info;

    let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
      layout: &HighLighter::layout(&gpu.device),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: self.lighter.data.as_bindable(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: self.mask.as_bindable(),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: resources
            .samplers
            .retrieve(&gpu.device, &TextureSampler::default())
            .as_bindable(),
        },
      ],
      label: None,
    });
    self.bindgroup = Some(bindgroup);

    let mut hasher = Default::default();

    TypeId::of::<HighLighter>().hash(&mut hasher);
    pass_info.format_info.hash(&mut hasher);

    self.pipeline = resources
      .pipeline_resource
      .get_or_insert_with(hasher, || {
        HighLighter::build_pipeline(&gpu.device, &resources.layouts, &pass_info.format_info)
      })
      .clone()
      .into();
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, _scene: &'a Scene) {
    pass.set_pipeline(self.pipeline.as_ref().unwrap());
    pass.set_bind_group(0, self.bindgroup.as_ref().unwrap(), &[]);
    pass.set_bind_group_placeholder(1);
    pass.set_bind_group_placeholder(2);
    pass.draw(0..4, 0..1);
  }
}

impl HighLighter {
  fn build_pipeline(
    device: &wgpu::Device,
    layouts: &BindGroupLayoutCache,
    format_info: &PassTargetFormatInfo,
  ) -> wgpu::RenderPipeline {
    let mut builder = PipelineBuilder::default();

    full_screen_vertex_shader(
      &mut builder,
      wgpu::BlendState::ALPHA_BLENDING.into(),
      format_info,
    );

    builder
      .with_layout::<HighLighter>(layouts, device)
      .with_layout::<PassGPUData>(layouts, device)
      .include_fragment_entry(
        "
    [[stage(fragment)]]
    fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
      var x_step: f32 = pass_info.texel_size.x * highlighter.width;
      var y_step: f32 = pass_info.texel_size.y * highlighter.width;

      var all: f32 = 0.0;
      all = all + textureSample(mask, tex_sampler, in.uv).x;
      all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y)).x;
      all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x, in.uv.y + y_step)).x;
      all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y+ y_step)).x;

      var intensity = (1.0 - 2.0 * abs(all / 4. - 0.5)) * highlighter.color.a;

      return vec4<f32>(highlighter.color.rgb, intensity);
    }}
    
    ",
      )
      .use_fragment_entry("fs_main");

    builder.build(device)
  }
}

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
    builder.fragment(|builder| {
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
        self.scene.active_camera.as_ref().unwrap() & self.scene.resources,
      )
    }
  }
}
