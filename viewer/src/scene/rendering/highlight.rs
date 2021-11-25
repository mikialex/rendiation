use crate::{
  AttachmentOwnedReadView, MeshModel, PassContent, PassDispatcher, Scene, SceneRenderable,
};

use rendiation_algebra::Vec4;
use rendiation_texture::TextureSampler;
use rendiation_webgpu::*;

pub struct HighLighter {
  pub color: UniformBufferData<Vec4<f32>>,
}

impl HighLighter {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      color: UniformBufferData::create(&gpu.device, (0., 0.8, 1., 1.).into()),
    }
  }
}

impl HighLighter {
  pub fn draw(&self, mask: AttachmentOwnedReadView<wgpu::TextureFormat>) -> HighLightComposeTask {
    HighLightComposeTask {
      mask,
      lighter: self,
      bindgroup: None,
    }
  }
}

pub struct HighLightComposeTask<'a> {
  mask: AttachmentOwnedReadView<wgpu::TextureFormat>,
  lighter: &'a HighLighter,
  bindgroup: Option<wgpu::BindGroup>,
}

impl BindGroupLayoutProvider for HighLighter {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: UniformBuffer::<Vec4<f32>>::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: WebGPUTexture2d::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
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
      [[block]]
      struct HighLighter {{
        color: vec4<f32>;
      }};

      [[group({group}), binding(0)]]
      var<uniform> highlighter: HighLighter;
      
      [[group({group}), binding(1)]]
      var mask: texture_2d<f32>;

      [[group({group}), binding(2)]]
      var sampler: sampler;
    "
    )
  }
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo) {
    let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
      layout: &HighLighter::layout(&gpu.device),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: self.lighter.color.gpu().as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: self.mask.as_bindable(),
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

    // todo pipeline
  }

  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene) {
    // pass.set
  }
}

struct HighLightComposer {
  buffer: UniformBuffer<Vec4<f32>>,
  bindgroup: BindGroup,
}

impl HighLightComposer {
  fn build_pipeline(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    let mut builder = PipelineBuilder::default();

    builder.include_fragment_entry(
      "
    const CIRCLE_SAMPLES  = 32

    [[stage(fragment)]]
    fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
        return textureSample(r_color, r_sampler, in.uv);
    }}
    ",
    );

    builder.build(device)
  }
}

pub struct HighLightDrawMaskTask<T> {
  objects: T,
}

pub fn highlight<T>(objects: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { objects }
}

struct HighLightMaskDispatcher;

impl PassDispatcher for HighLightMaskDispatcher {
  fn build_pipeline(&self, builder: &mut PipelineBuilder) {
    todo!()
  }
}

impl<'i, T> PassContent for HighLightDrawMaskTask<T>
where
  T: IntoIterator<Item = &'i MeshModel> + Copy,
{
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo) {
    let mut base = scene.create_material_ctx_base(gpu, pass_info, &HighLightMaskDispatcher);

    for model in self.objects {
      let mut model = model.inner.borrow_mut();
      model.update(gpu, &mut base);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene) {
    for model in self.objects {
      model.setup_pass(
        pass,
        scene.active_camera.as_ref().unwrap().expect_gpu(),
        &scene.resources,
      )
    }
  }
}
