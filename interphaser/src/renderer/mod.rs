use std::sync::Arc;

use bytemuck::*;
use rendiation_algebra::*;
use rendiation_shader_api::{std140_layout, GraphicsShaderProvider, ShaderStruct};
use rendiation_shader_backend_naga::ShaderAPINagaImpl;
use rendiation_texture::Size;
use webgpu::util::DeviceExt;
use webgpu::*;

mod pipeline;
use pipeline::*;

pub mod text;
pub use text::*;

pub mod graphics;
pub use graphics::*;

use super::{Primitive, UIPresentation};
use crate::*;

pub struct WebGpuUIPresenter {
  pub(crate) gpu: Arc<GPU>,
  surface: GPUSurface,
  ui_renderer: WebGPUxUIRenderer,
}

impl WebGpuUIPresenter {
  pub async fn new(window: &winit::window::Window) -> Self {
    let mut minimal_required_features = Features::all_webgpu_mask();

    // minimal_required_features.insert(Features::TEXTURE_BINDING_ARRAY);
    // minimal_required_features.insert(Features::BUFFER_BINDING_ARRAY);
    // minimal_required_features.insert(Features::PARTIALLY_BOUND_BINDING_ARRAY);

    minimal_required_features.remove(Features::TIMESTAMP_QUERY); // note: on macos we currently do not have this

    let config = GPUCreateConfig {
      backends: Backends::PRIMARY,
      power_preference: PowerPreference::HighPerformance,
      surface_for_compatible_check_init: Some((window, Size::from_usize_pair_min_one((300, 200)))),
      minimal_required_features,
      minimal_required_limits: Limits::default(),
    };

    let (gpu, surface) = GPU::new(config).await.unwrap();
    let gpu = Arc::new(gpu);
    let surface = surface.unwrap();

    let prefer_target_fmt = surface.config.format;
    let ui_renderer = WebGPUxUIRenderer::new(&gpu.device, prefer_target_fmt, TEXT_CACHE_INIT_SIZE);

    Self {
      surface,
      gpu,
      ui_renderer,
    }
  }
}

impl UIPresenter for WebGpuUIPresenter {
  fn resize(&mut self, size: Size) {
    self.surface.resize(size, &self.gpu.device);
  }

  fn render(&mut self, presentation: &UIPresentation, fonts: &FontManager, texts: &mut TextCache) {
    if let Ok((frame, view)) = self.surface.get_current_frame_with_render_target_view() {
      self.gpu.poll();

      let mut task = WebGPUxUIRenderTask {
        fonts,
        renderer: &mut self.ui_renderer,
        presentation,
      };

      let mut encoder = self.gpu.create_encoder();
      task.update(&self.gpu, &mut encoder, fonts, texts);

      let mut decs = RenderPassDescriptorOwned::default();
      decs.channels.push((
        webgpu::Operations {
          load: webgpu::LoadOp::Clear(webgpu::Color::WHITE),
          store: true,
        },
        view,
      ));
      {
        let mut pass = encoder.begin_render_pass(decs);
        task.setup_pass(&mut pass);
      }
      self.gpu.submit_encoder(encoder);

      frame.present();
    }
  }
}

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
        pass.set_pipeline(&renderer.resource.solid_color_pipeline.pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_index_buffer(p.index_buffer.slice(..), webgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
      GPUxUIPrimitive::Texture(tex) => {
        pass.set_pipeline(&renderer.resource.texture_pipeline.pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_bind_group(1, &tex.bindgroup, &[]);
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
  bindgroup: webgpu::BindGroup,
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
  quad: &crate::RectangleShape,
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
    device: &webgpu::GPUDevice,
    _encoder: &mut GPUCommandEncoder,
    res: &UIxGPUxResource,
    texts: &mut TextCache,
  ) -> Option<GPUxUIPrimitive> {
    let p = match self {
      Primitive::Quad((quad, style)) => match style {
        crate::Style::SolidColor(color) => {
          let color = *color;

          let (index_buffer, vertex_buffer) = build_quad(device, quad, color);

          GPUxUIPrimitive::SolidColor(GPUxUISolidColorPrimitive {
            vertex_buffer,
            index_buffer,
            length: 6,
          })
        }
        crate::Style::Texture(view) => {
          let (index_buffer, vertex_buffer) = build_quad(device, quad, (1., 1., 1., 1.).into());

          let bindgroup = device.create_bind_group(&webgpu::BindGroupDescriptor {
            label: None,
            layout: &res.texture_bg_layout,
            entries: &[
              webgpu::BindGroupEntry {
                binding: 0,
                resource: view.as_bindable(),
              },
              webgpu::BindGroupEntry {
                binding: 1,
                resource: res.sampler.as_bindable(),
              },
            ],
          });

          GPUxUIPrimitive::Texture(GPUxUITexturedPrimitive {
            vertex_buffer,
            index_buffer,
            length: 6,
            bindgroup,
          })
        }
      },
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
  global_ui_state: UniformBufferDataView<UIGlobalParameter>,
  texture_bg_layout: webgpu::GPUBindGroupLayout,
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
      ..Zeroable::zeroed()
    };

    let global_ui_state = UniformBufferDataView::create(device, global_ui_state);
    let global_uniform_bind_group_layout = UIGlobalParameter::create_bind_group_layout(device);

    let global_bindgroup = device.create_bind_group(&webgpu::BindGroupDescriptor {
      layout: &global_uniform_bind_group_layout,
      entries: &[webgpu::BindGroupEntry {
        binding: 0,
        resource: global_ui_state.as_bindable(),
      }],
      label: None,
    });

    let solid_color_pipeline = device
      .build_pipeline_by_shader_api(
        SolidUIPipeline { target_format }
          .build_self(
            Box::new(ShaderAPINagaImpl::new(
              rendiation_shader_api::ShaderStages::Vertex,
            )),
            Box::new(ShaderAPINagaImpl::new(
              rendiation_shader_api::ShaderStages::Fragment,
            )),
          )
          .unwrap(),
      )
      .unwrap();

    let texture_pipeline = device
      .build_pipeline_by_shader_api(
        TextureUIPipeline { target_format }
          .build_self(
            Box::new(ShaderAPINagaImpl::new(
              rendiation_shader_api::ShaderStages::Vertex,
            )),
            Box::new(ShaderAPINagaImpl::new(
              rendiation_shader_api::ShaderStages::Fragment,
            )),
          )
          .unwrap(),
      )
      .unwrap();

    let texture_bg_layout = texture_pipeline.get_layout(1).clone();

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

    let screen_size = Vec2::new(presentation.view_size.width, presentation.view_size.height);

    self
      .resource
      .global_ui_state
      .mutate(|t| t.screen_size = screen_size);

    self.resource.global_ui_state.upload(&gpu.queue);

    self.text_renderer.resize_view(screen_size, &gpu.queue);

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
#[std140_layout]
#[derive(Debug, Copy, Clone, ShaderStruct)]
pub struct UIGlobalParameter {
  pub screen_size: Vec2<f32>,
}

impl UIGlobalParameter {
  fn create_bind_group_layout(device: &webgpu::Device) -> webgpu::BindGroupLayout {
    device.create_bind_group_layout(&webgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[webgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: webgpu::ShaderStages::VERTEX_FRAGMENT,
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
