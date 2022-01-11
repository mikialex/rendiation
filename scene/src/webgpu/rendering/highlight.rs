use std::{any::TypeId, hash::Hash, rc::Rc};

use crate::{
  full_screen_vertex_shader, AttachmentOwnedReadView, PassContent, PassDispatcher, PassGPUData,
  PassUpdateCtx, RenderPassGPUInfoData, Scene, SceneRenderPass, SceneRenderable,
};

use rendiation_algebra::*;
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
  pub _pad: Vec3<f32>,
}

impl ShaderUniformBlock for HighLightData {
  fn shader_struct() -> &'static str {
    "
    struct HighLightData {
      color: vec4<f32>;
      width: f32;
    };
  "
  }
}

impl Default for HighLightData {
  fn default() -> Self {
    Self {
      color: (0., 0.4, 8., 1.).into(),
      width: 2.,
      _pad: Default::default(),
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

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder
      .declare_uniform_struct::<HighLightData>()
      .declare_uniform_struct::<RenderPassGPUInfoData>();
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var<uniform> highlighter: HighLightData;
      
      [[group({group}), binding(1)]]
      var mask: texture_2d<f32>;

      [[group({group}), binding(2)]]
      var tex_sampler: sampler;
    "
    )
  }
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
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
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    let mut base = scene.create_material_ctx_base(gpu, ctx.pass_info, &HighLightMaskDispatcher);

    for model in self.objects {
      model.update(gpu, &mut base);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    for model in self.objects {
      model.setup_pass(
        pass,
        scene
          .resources
          .cameras
          .expect_gpu(scene.active_camera.as_ref().unwrap()),
        &scene.resources,
      )
    }
  }
}
