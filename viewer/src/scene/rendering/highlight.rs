use std::{any::TypeId, hash::Hash, rc::Rc};

use crate::{
  full_screen_vertex_shader, AttachmentOwnedReadView, MaterialStates, MeshModel, PassContent,
  PassDispatcher, RenderPassGPUInfoData, Scene, SceneRenderable,
};

use rendiation_algebra::{Vec2, Vec4};
use rendiation_texture::TextureSampler;
use rendiation_webgpu::*;

pub struct HighLighter {
  pub data: UniformBufferData<HighLightData>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
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

impl BindGroupLayoutProvider for HighLighter {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: UniformBuffer::<Vec4<f32>>::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: UniformBuffer::<RenderPassGPUInfoData>::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: WebGPUTexture2d::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 3,
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
      struct HighLightData {{
        color: vec4<f32>;
        width: f32;
      }};

      
      [[block]]
      struct RenderPassGPUInfoData {{
        texel_size:  vec2<f32>;
      }};

      [[group({group}), binding(0)]]
      var<uniform> highlighter: HighLightData;
      
      [[group({group}), binding(1)]]
      var<uniform> pass_info: RenderPassGPUInfoData;
      
      [[group({group}), binding(2)]]
      var mask: texture_2d<f32>;

      [[group({group}), binding(3)]]
      var sampler: sampler;
    "
    )
  }
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo) {
    self.lighter.data.update(&gpu.queue);

    // we should create new buffer  every frame.
    let buffer_size = pass_info.buffer_size.into_usize();
    let buffer_size: Vec2<f32> = (buffer_size.0 as f32, buffer_size.1 as f32).into();
    let pass_info_gpu = UniformBuffer::create(
      &gpu.device,
      RenderPassGPUInfoData {
        texel_size: buffer_size.map(|v| 1. / v),
      },
    );

    let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
      layout: &HighLighter::layout(&gpu.device),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: self.lighter.data.gpu().as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: pass_info_gpu.gpu().as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: self.mask.as_bindable(),
        },
        wgpu::BindGroupEntry {
          binding: 3,
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

    TypeId::of::<HighLighter>().hash(&mut hasher);
    pass_info.format_info.hash(&mut hasher);

    self.pipeline = scene
      .resources
      .pipeline_resource
      .get_or_insert_with(hasher, || {
        HighLighter::build_pipeline(
          &gpu.device,
          &scene.resources.layouts,
          &pass_info.format_info,
        )
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

impl HighLighter {
  fn build_pipeline(
    device: &wgpu::Device,
    layouts: &BindGroupLayoutCache,
    format_info: &PassTargetFormatInfo,
  ) -> wgpu::RenderPipeline {
    let mut builder = PipelineBuilder::default();
    builder.primitive_state = wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleStrip,
      front_face: wgpu::FrontFace::Cw,
      ..Default::default()
    };

    full_screen_vertex_shader(&mut builder);
    builder
      .with_layout::<HighLighter>(layouts, device)
      .declare_struct(
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
      var x_step: f32 = pass_info.texel_size.x * highlighter.width;
      var y_step: f32 = pass_info.texel_size.y * highlighter.width;

      var all: f32 = 0.0;
      all = all + textureSample(mask, sampler, in.uv).x;
      all = all + textureSample(mask, sampler, vec2<f32>(in.uv.x + x_step, in.uv.y)).x;
      all = all + textureSample(mask, sampler, vec2<f32>(in.uv.x, in.uv.y + y_step)).x;
      all = all + textureSample(mask, sampler, vec2<f32>(in.uv.x + x_step, in.uv.y+ y_step)).x;

      var intensity = (1.0 - 2.0 * abs(all / 4. - 0.5)) * highlighter.color.a;

      return vec4<f32>(highlighter.color.rgb, intensity);
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
    .apply_pipeline_builder(&mut builder, format_info);

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
    builder
      .include_fragment_entry(
        "
    [[stage(fragment)]]
    fn fs_highlight_mask_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
        return vec4<f32>(1.);
    }}
    ",
      )
      .use_fragment_entry("fs_highlight_mask_main");
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
