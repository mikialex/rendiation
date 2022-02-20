use bytemuck::*;
use rendiation_algebra::*;
use rendiation_texture::Size;
use shadergraph::ShaderUniform;
use webgpu::util::DeviceExt;
use webgpu::*;

mod pipeline;
use pipeline::*;

pub mod text;
pub use text::*;

pub mod graphics;
pub use graphics::*;

use crate::{FontManager, TextCache, TextHash};

use super::{Primitive, UIPresentation};

pub struct WebGPUxUIRenderTask<'a> {
  pub renderer: &'a mut WebGPUxUIRenderer,
  pub fonts: &'a FontManager,
  pub presentation: &'a UIPresentation,
}

impl<'r> WebGPUxUIRenderTask<'r> {
  pub fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>) {
    let renderer = &self.renderer;
    renderer.gpu_primitive_cache.iter().for_each(|p| match p {
      GPUxUIPrimitive::SolidColor(p) => {
        pass.set_pipeline(&renderer.resource.solid_color_pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_index_buffer(p.index_buffer.slice(..), webgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
      GPUxUIPrimitive::Texture(tex) => {
        pass.set_pipeline(&renderer.resource.texture_pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_bind_group(1, &tex.bindgroup.bindgroup, &[]);
        pass.set_index_buffer(tex.index_buffer.slice(..), webgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, tex.vertex_buffer.slice(..));
        pass.draw_indexed(0..tex.length, 0, 0..1);
      }
      GPUxUIPrimitive::Text(text) => {
        self.renderer.text_renderer.draw_gpu_text(pass, *text);
      }
    });
  }

  pub fn update(
    &mut self,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    fonts: &FontManager,
    texts: &mut TextCache,
  ) {
    self
      .renderer
      .update(self.presentation, gpu, encoder, fonts, texts)
  }
}

pub struct GPUxUISolidColorPrimitive {
  // uniform: webgpu::Buffer,
  // bindgroup: webgpu::BindGroup,
  vertex_buffer: webgpu::Buffer,
  index_buffer: webgpu::Buffer,
  length: u32,
}

pub struct GPUxUITexturedPrimitive {
  bindgroup: TextureBindGroup,
  vertex_buffer: webgpu::Buffer,
  index_buffer: webgpu::Buffer,
  length: u32,
}

pub enum GPUxUIPrimitive {
  SolidColor(GPUxUISolidColorPrimitive),
  Texture(GPUxUITexturedPrimitive),
  Text(TextHash),
}

#[allow(clippy::vec_init_then_push)]
fn build_quad(
  device: &webgpu::Device,
  quad: &crate::Quad,
  color: crate::Color,
) -> (webgpu::Buffer, webgpu::Buffer) {
  let mut vertices = Vec::new();

  #[rustfmt::skip]
  {
  vertices.push(vertex((quad.x, quad.y), (0., 0.), color.into()));
  vertices.push(vertex((quad.x, quad.y + quad.height), (0., 1.), color.into()));
  vertices.push(vertex((quad.x + quad.width, quad.y), (1., 0.), color.into()));
  vertices.push(vertex((quad.x + quad.width, quad.y + quad.height), (1., 1.), color.into()));
  }
  let mut index = Vec::<u32>::new();
  index.push(0);
  index.push(1);
  index.push(2);
  index.push(2);
  index.push(1);
  index.push(3);

  let vertex = bytemuck::cast_slice(vertices.as_slice());
  let vertex_buffer = device.create_buffer_init(&webgpu::util::BufferInitDescriptor {
    label: None,
    contents: vertex,
    usage: webgpu::BufferUsages::VERTEX,
  });

  let index = bytemuck::cast_slice(index.as_slice());
  let index_buffer = device.create_buffer_init(&webgpu::util::BufferInitDescriptor {
    label: None,
    contents: index,
    usage: webgpu::BufferUsages::INDEX,
  });

  (index_buffer, vertex_buffer)
}

impl Primitive {
  pub fn create_gpu(
    &self,
    device: &webgpu::Device,
    _encoder: &mut GPUCommandEncoder,
    res: &UIxGPUxResource,
    texts: &mut TextCache,
  ) -> Option<GPUxUIPrimitive> {
    let p = match self {
      Primitive::Quad((quad, style)) => {
        match style {
          crate::Style::SolidColor(color) => {
            let color = *color;

            // let index_mesh: UIMesh = IndexedMesh::new(vertices, index);
            let (index_buffer, vertex_buffer) = build_quad(device, quad, color);

            GPUxUIPrimitive::SolidColor(GPUxUISolidColorPrimitive {
              vertex_buffer,
              index_buffer,
              length: 6,
            })
          }
          crate::Style::Texture(view) => {
            let (index_buffer, vertex_buffer) = build_quad(device, quad, (1., 1., 1., 1.).into());

            GPUxUIPrimitive::Texture(GPUxUITexturedPrimitive {
              vertex_buffer,
              index_buffer,
              length: 6,
              bindgroup: TextureBindGroup::new(
                device,
                &res.texture_bg_layout,
                &res.sampler,
                view.as_ref(),
              ),
            })
          }
        }
      }
      Primitive::Text(text) => {
        texts.queue(text.hash());
        GPUxUIPrimitive::Text(text.hash())
      }
    };
    p.into()
  }
}

pub struct WebGPUxUIRenderer {
  gpu_primitive_cache: Vec<GPUxUIPrimitive>,
  resource: UIxGPUxResource,
  text_renderer: TextRenderer,
}

pub struct UIxGPUxResource {
  solid_color_pipeline: webgpu::GPURenderPipeline,
  texture_pipeline: webgpu::GPURenderPipeline,
  global_ui_state: UniformBufferData<UIGlobalParameter>,
  texture_bg_layout: webgpu::BindGroupLayout,
  sampler: webgpu::Sampler,
  global_bindgroup: webgpu::BindGroup,
}

impl WebGPUxUIRenderer {
  pub fn new(
    device: &webgpu::GPUDevice,
    target_format: webgpu::TextureFormat,
    text_cache_init_size: Size,
  ) -> Self {
    let global_ui_state = UIGlobalParameter {
      screen_size: Vec2::new(1000., 1000.),
    };

    let global_ui_state = UniformBufferData::create(device, global_ui_state);
    let global_uniform_bind_group_layout = UIGlobalParameter::create_bind_group_layout(device);

    let global_bindgroup = device.create_bind_group(&webgpu::BindGroupDescriptor {
      layout: &global_uniform_bind_group_layout,
      entries: &[webgpu::BindGroupEntry {
        binding: 0,
        resource: global_ui_state.as_bindable(),
      }],
      label: None,
    });

    let texture_bg_layout = TextureBindGroup::create_bind_group_layout(device);

    let solid_color_pipeline = device
      .build_pipeline_by_shadergraph(&SolidUIPipeline { target_format })
      .unwrap();

    let texture_pipeline = device
      .build_pipeline_by_shadergraph(&TextureUIPipeline { target_format })
      .unwrap();

    let text_renderer = TextRenderer::new(
      device,
      webgpu::FilterMode::Linear,
      target_format,
      text_cache_init_size,
    );

    let sampler = device.create_sampler(&SamplerDescriptor {
      address_mode_u: webgpu::AddressMode::ClampToEdge,
      address_mode_v: webgpu::AddressMode::ClampToEdge,
      address_mode_w: webgpu::AddressMode::ClampToEdge,
      mag_filter: webgpu::FilterMode::Nearest,
      min_filter: webgpu::FilterMode::Nearest,
      mipmap_filter: webgpu::FilterMode::Nearest,
      ..Default::default()
    });

    let resource = UIxGPUxResource {
      solid_color_pipeline,
      texture_pipeline,
      global_ui_state,
      texture_bg_layout,
      sampler,
      global_bindgroup,
    };

    Self {
      gpu_primitive_cache: Vec::new(),
      resource,
      text_renderer,
    }
  }

  fn update(
    &mut self,
    presentation: &UIPresentation,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    fonts: &FontManager,
    texts: &mut TextCache,
  ) {
    self.gpu_primitive_cache.clear();

    self.resource.global_ui_state.screen_size =
      Vec2::new(presentation.view_size.width, presentation.view_size.height);
    self.resource.global_ui_state.update(&gpu.queue);

    self
      .text_renderer
      .resize_view(self.resource.global_ui_state.screen_size, &gpu.queue);

    self.gpu_primitive_cache.extend(
      presentation
        .primitives
        .iter()
        .filter_map(|p| p.create_gpu(&gpu.device, encoder, &self.resource, texts)),
    );

    self.text_renderer.process_queued(gpu, fonts, texts);
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, ShaderUniform, Zeroable, Pod)]
struct UIGlobalParameter {
  pub screen_size: Vec2<f32>,
}

impl UIGlobalParameter {
  fn create_bind_group_layout(device: &webgpu::Device) -> webgpu::BindGroupLayout {
    device.create_bind_group_layout(&webgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[webgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: webgpu::ShaderStages::VERTEX,
        ty: webgpu::BindingType::Buffer {
          has_dynamic_offset: false,
          min_binding_size: None,
          ty: webgpu::BufferBindingType::Uniform,
        },
        count: None,
      }],
    })
  }
}
