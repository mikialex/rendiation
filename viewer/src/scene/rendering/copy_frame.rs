use std::{any::TypeId, hash::Hash, rc::Rc};

use rendiation_texture::TextureSampler;
use rendiation_webgpu::{
  BindGroupDescriptor, BindGroupLayoutProvider, BindableResource, GPURenderPass, PipelineBuilder,
  RenderPassInfo, WebGPUTexture2d, GPU,
};

use crate::{
  full_screen_vertex_shader, AttachmentOwnedReadView, MaterialStates, PassContent, Scene,
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
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo) {
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

    TypeId::of::<Self>().hash(&mut hasher);
    pass_info.format_info.hash(&mut hasher);

    self.pipeline = scene
      .resources
      .pipeline_resource
      .get_or_insert_with(hasher, || {
        let mut builder = PipelineBuilder::default();
        builder.primitive_state = wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleStrip,
          front_face: wgpu::FrontFace::Cw,
          ..Default::default()
        };

        full_screen_vertex_shader(&mut builder);
        builder
          .with_layout::<Self>(&scene.resources.layouts, &gpu.device)
          .declare_io_struct(
            "
            struct VertexOutput {
              [[builtin(position)]] position: vec4<f32>;
              [[location(0)]] uv: vec2<f32>;
            };
          ",
          )
          .include_fragment_entry(
            "
          [[stage(fragment)]]
          fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
            return textureSample(texture, sampler, in.uv);
          }}
          ",
          )
          .use_fragment_entry("fs_main");

        MaterialStates {
          blend: wgpu::BlendState::ALPHA_BLENDING.into(),
          depth_write_enabled: false,
          depth_compare: wgpu::CompareFunction::Always,
          ..Default::default()
        }
        .apply_pipeline_builder(&mut builder, &pass_info.format_info);

        builder.build(&gpu.device)
      })
      .clone()
      .into();
  }

  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, _scene: &'a Scene) {
    pass.set_pipeline(self.pipeline.as_ref().unwrap());
    pass.set_bind_group(0, self.bindgroup.as_ref().unwrap(), &[]);
    pass.draw(0..4, 0..1);
  }
}

impl BindGroupLayoutProvider for CopyFrame {
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
