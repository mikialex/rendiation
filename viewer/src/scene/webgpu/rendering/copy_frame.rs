use std::{any::TypeId, hash::Hash, rc::Rc};

use rendiation_texture::TextureSampler;
use rendiation_webgpu::{
  BindGroupDescriptor, BindGroupLayoutProvider, BindableResource, PipelineBuilder, WebGPUTexture2d,
  GPU,
};

use crate::{
  full_screen_vertex_shader, AttachmentOwnedReadView, PassContent, PassUpdateCtx, Scene,
  SceneRenderPass,
};

pub struct CopyFrame {
  source: AttachmentOwnedReadView<wgpu::TextureFormat>,
  bindgroup: Option<wgpu::BindGroup>,
  pipeline: Option<Rc<wgpu::RenderPipeline>>,
}

pub fn copy_frame(source: AttachmentOwnedReadView<wgpu::TextureFormat>) -> CopyFrame {
  CopyFrame {
    source,
    bindgroup: None,
    pipeline: None,
  }
}

impl PassContent for CopyFrame {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
      layout: &Self::layout(&gpu.device),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: self.source.as_bindable(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: scene
            .resources
            .samplers
            .retrieve(&gpu.device, &TextureSampler::default())
            .as_bindable(),
        },
      ],
      label: None,
    });
    self.bindgroup = Some(bindgroup);

    let mut hasher = Default::default();

    let pass_info = ctx.pass_info;

    TypeId::of::<Self>().hash(&mut hasher);
    pass_info.format_info.hash(&mut hasher);

    self.pipeline = scene
      .resources
      .pipeline_resource
      .get_or_insert_with(hasher, || {
        let mut builder = PipelineBuilder::default();

        full_screen_vertex_shader(
          &mut builder,
          wgpu::BlendState::ALPHA_BLENDING.into(),
          &pass_info.format_info,
        );

        builder
          .with_layout::<Self>(&scene.resources.layouts, &gpu.device)
          .include_fragment_entry(
            "
          [[stage(fragment)]]
          fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
            return textureSample(texture, sampler, in.uv);
          }}
          ",
          )
          .use_fragment_entry("fs_main");

        builder.build(&gpu.device)
      })
      .clone()
      .into();
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, _scene: &'a Scene) {
    pass.set_pipeline(self.pipeline.as_ref().unwrap());
    pass.set_bind_group(0, self.bindgroup.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..1);
  }
}

impl BindGroupLayoutProvider for CopyFrame {
  fn bind_preference() -> usize {
    0
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: WebGPUTexture2d::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::Sampler::bind_layout(),
          count: None,
        },
      ],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
        [[group({group}), binding(0)]]
        var texture: texture_2d<f32>;
  
        [[group({group}), binding(1)]]
        var sampler: sampler;
      "
    )
  }

  fn register_uniform_struct_declare(_: &mut PipelineBuilder) {}
}
