use rendiation_algebra::*;
use rendiation_texture::Size;
use rendiation_webgpu::*;
use wgpu::util::DeviceExt;

mod pipeline;
use pipeline::*;

pub mod text;
pub use text::*;

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
        pass.set_index_buffer(p.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
      GPUxUIPrimitive::Texture(tex) => {
        pass.set_pipeline(&renderer.resource.texture_pipeline);
        pass.set_bind_group(0, &self.renderer.resource.global_bindgroup, &[]);
        pass.set_bind_group(1, &tex.bindgroup.bindgroup, &[]);
        pass.set_index_buffer(tex.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
  // uniform: wgpu::Buffer,
  // bindgroup: wgpu::BindGroup,
  vertex_buffer: wgpu::Buffer,
  index_buffer: wgpu::Buffer,
  length: u32,
}

pub struct GPUxUITexturedPrimitive {
  bindgroup: TextureBindGroup,
  vertex_buffer: wgpu::Buffer,
  index_buffer: wgpu::Buffer,
  length: u32,
}

pub enum GPUxUIPrimitive {
  SolidColor(GPUxUISolidColorPrimitive),
  Texture(GPUxUITexturedPrimitive),
  Text(TextHash),
}

#[allow(clippy::vec_init_then_push)]
fn build_quad(
  device: &wgpu::Device,
  quad: &crate::Quad,
  color: crate::Color,
) -> (wgpu::Buffer, wgpu::Buffer) {
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
  let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: vertex,
    usage: wgpu::BufferUsages::VERTEX,
  });

  let index = bytemuck::cast_slice(index.as_slice());
  let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: index,
    usage: wgpu::BufferUsages::INDEX,
  });

  (index_buffer, vertex_buffer)
}

impl Primitive {
  pub fn create_gpu(
    &self,
    device: &wgpu::Device,
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
  solid_color_pipeline: wgpu::RenderPipeline,
  texture_pipeline: wgpu::RenderPipeline,
  global_ui_state: UniformBufferData<UIGlobalParameter>,
  texture_bg_layout: wgpu::BindGroupLayout,
  sampler: wgpu::Sampler,
  global_bindgroup: wgpu::BindGroup,
}

impl WebGPUxUIRenderer {
  pub fn new(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    text_cache_init_size: Size,
  ) -> Self {
    let global_ui_state = UIGlobalParameter {
      screen_size: Vec2::new(1000., 1000.),
    };

    let global_ui_state = UniformBufferData::create(device, global_ui_state);
    let global_uniform_bind_group_layout = UIGlobalParameter::create_bind_group_layout(device);

    let global_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &global_uniform_bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: global_ui_state.as_bindable(),
      }],
      label: None,
    });

    let texture_bg_layout = TextureBindGroup::create_bind_group_layout(device);

    let solid_color_pipeline =
      create_solid_pipeline(device, target_format, &global_uniform_bind_group_layout);

    let texture_pipeline = create_texture_pipeline(
      device,
      target_format,
      &global_uniform_bind_group_layout,
      &texture_bg_layout,
    );

    let text_renderer = TextRenderer::new(
      device,
      wgpu::FilterMode::Linear,
      wgpu::TextureFormat::Bgra8UnormSrgb,
      text_cache_init_size,
    );

    let sampler = device.create_sampler(&SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
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

#[derive(Debug, Copy, Clone)]
struct UIGlobalParameter {
  pub screen_size: Vec2<f32>,
}

impl ShaderUniformBlock for UIGlobalParameter {
  fn shader_struct() -> &'static str {
    "
       [[block]] struct UIGlobalParameter {
        screen_size: vec2<f32>;
      };
       "
  }
}

unsafe impl bytemuck::Zeroable for UIGlobalParameter {}
unsafe impl bytemuck::Pod for UIGlobalParameter {}

impl UIGlobalParameter {
  fn get_shader_header() -> &'static str {
    "
    [[block]] struct UIGlobalParameter {
      screen_size: vec2<f32>;
    };
    [[group(0), binding(0)]] 
    var<uniform> ui_global_parameter: UIGlobalParameter;
    "
  }

  fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          has_dynamic_offset: false,
          min_binding_size: None,
          ty: wgpu::BufferBindingType::Uniform,
        },
        count: None,
      }],
    })
  }
}

#[derive(Debug, Copy, Clone)]
pub struct UIVertex {
  position: Vec2<f32>,
  uv: Vec2<f32>,
  color: Vec4<f32>,
}
unsafe impl bytemuck::Zeroable for UIVertex {}
unsafe impl bytemuck::Pod for UIVertex {}

fn vertex(position: (f32, f32), uv: (f32, f32), color: (f32, f32, f32, f32)) -> UIVertex {
  UIVertex {
    position: position.into(),
    uv: uv.into(),
    color: color.into(),
  }
}

impl VertexBufferSourceType for UIVertex {
  fn vertex_layout() -> VertexBufferLayoutOwned {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<UIVertex>() as u64,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
      ],
    }
    .into()
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]] position: vec2<f32>,
      [[location(1)]] uv: vec2<f32>,
      [[location(2)]] color: vec4<f32>,
    "#
  }
}
