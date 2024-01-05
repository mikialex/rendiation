use std::sync::Arc;

use bytemuck::*;
use rendiation_algebra::*;
use rendiation_shader_api::{std140_layout, GraphicsShaderProvider, ShaderStruct};
use rendiation_shader_backend_naga::ShaderAPINagaImpl;
use rendiation_texture::Size;

mod pipeline;
use pipeline::*;

mod text;
pub use text::*;

mod graphics;
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
      surface_for_compatible_check_init: Some((window, Size::from_usize_pair_min_one((300, 200)))),
      minimal_required_features,
      ..Default::default()
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
        Operations {
          load: LoadOp::Clear(rendiation_webgpu::Color::WHITE),
          store: StoreOp::Store,
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
        pass.set_index_buffer(p.index_buffer.slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
      GPUxUIPrimitive::Texture(tex) => {
        pass.set_pipeline(&renderer.resource.texture_pipeline.pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_bind_group(1, &tex.bindgroup, &[]);
        pass.set_index_buffer(tex.index_buffer.slice(..), IndexFormat::Uint32);
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
  // uniform: Buffer,
  // bindgroup: BindGroup,
  vertex_buffer: Buffer,
  index_buffer: Buffer,
  length: u32,
}

pub struct GPUxUITexturedPrimitive {
  bindgroup: BindGroup,
  vertex_buffer: Buffer,
  index_buffer: Buffer,
  length: u32,
}

pub enum GPUxUIPrimitive {
  SolidColor(GPUxUISolidColorPrimitive),
  Texture(GPUxUITexturedPrimitive),
  Text(TextHash),
}

fn build_quad(
  device: &Device,
  quad: &crate::RectangleShape,
  color: DisplayColor,
) -> (Buffer, Buffer) {
  let mut vertices = Vec::with_capacity(4);

  #[rustfmt::skip]
  {
    vertices.push(vertex((quad.x, quad.y), (0., 0.), color.into()));
    vertices.push(vertex((quad.x, quad.y + quad.height), (0., 1.), color.into()));
    vertices.push(vertex((quad.x + quad.width, quad.y), (1., 0.), color.into()));
    vertices.push(vertex((quad.x + quad.width, quad.y + quad.height), (1., 1.), color.into()));
  }
  let index: Vec<u32> = vec![0, 1, 2, 2, 1, 3];

  let vertex = bytemuck::cast_slice(vertices.as_slice());
  let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
    label: None,
    contents: vertex,
    usage: BufferUsages::VERTEX,
  });

  let index = bytemuck::cast_slice(index.as_slice());
  let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
    label: None,
    contents: index,
    usage: BufferUsages::INDEX,
  });

  (index_buffer, vertex_buffer)
}

impl Primitive {
  pub fn create_gpu(
    &self,
    device: &GPUDevice,
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

          let bindgroup = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &res.texture_bg_layout,
            entries: &[
              BindGroupEntry {
                binding: 0,
                resource: view.as_bindable(),
              },
              BindGroupEntry {
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
  solid_color_pipeline: GPURenderPipeline,
  texture_pipeline: GPURenderPipeline,
  global_ui_state: UniformBufferDataView<UIGlobalParameter>,
  texture_bg_layout: GPUBindGroupLayout,
  sampler: Sampler,
  global_bindgroup: BindGroup,
}

impl WebGPUxUIRenderer {
  pub fn new(device: &GPUDevice, target_format: TextureFormat, text_cache_init_size: Size) -> Self {
    let global_ui_state = UIGlobalParameter {
      screen_size: Vec2::new(1000., 1000.),
      ..Zeroable::zeroed()
    };

    let global_ui_state = UniformBufferDataView::create(device, global_ui_state);
    let global_uniform_bind_group_layout = UIGlobalParameter::create_bind_group_layout(device);

    let global_bindgroup = device.create_bind_group(&BindGroupDescriptor {
      layout: &global_uniform_bind_group_layout,
      entries: &[BindGroupEntry {
        binding: 0,
        resource: global_ui_state.as_bindable(),
      }],
      label: None,
    });

    let solid_color_pipeline = device
      .build_pipeline_by_shader_api(
        SolidUIPipeline { target_format }
          .build_self(&|stage| Box::new(ShaderAPINagaImpl::new(stage)))
          .unwrap(),
      )
      .unwrap();

    let texture_pipeline = device
      .build_pipeline_by_shader_api(
        TextureUIPipeline { target_format }
          .build_self(&|stage| Box::new(ShaderAPINagaImpl::new(stage)))
          .unwrap(),
      )
      .unwrap();

    let texture_bg_layout = texture_pipeline.get_layout(1).clone();

    let text_renderer = TextRenderer::new(
      device,
      FilterMode::Linear,
      target_format,
      text_cache_init_size,
    );

    let sampler = device.create_sampler(&SamplerDescriptor {
      address_mode_u: AddressMode::ClampToEdge,
      address_mode_v: AddressMode::ClampToEdge,
      address_mode_w: AddressMode::ClampToEdge,
      mag_filter: FilterMode::Nearest,
      min_filter: FilterMode::Nearest,
      mipmap_filter: FilterMode::Nearest,
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
  fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: None,
      entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: rendiation_webgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: BindingType::Buffer {
          has_dynamic_offset: false,
          min_binding_size: None,
          ty: BufferBindingType::Uniform,
        },
        count: None,
      }],
    })
  }
}
