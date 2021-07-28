use glyph_brush::{Section, Text};
use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{IndexedMesh, TriangleList};
use rendiation_webgpu::*;
use wgpu::util::DeviceExt;

mod pipeline;
mod text;
use pipeline::*;


use self::text::{GPUxUITextPrimitive, TextRenderer};

use super::{Primitive, UIPresentation};

pub struct WebGPUxUIRenderPass<'a> {
  pub renderer: &'a mut WebGPUxUIRenderer,
  pub presentation: &'a UIPresentation,
}

impl<'r> RenderPassCreator<wgpu::TextureView> for WebGPUxUIRenderPass<'r> {
  fn create<'a>(
    &'a self,
    view: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "ui pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl<'r> Renderable for WebGPUxUIRenderPass<'r> {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
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
        self.renderer.text_renderer.draw_gpu_text(pass, text);
      }
    });
  }

  fn update(&mut self, renderer: &GPU, encoder: &mut wgpu::CommandEncoder) {
    self.renderer.update(
      &self.presentation,
      &renderer.device,
      &renderer.queue,
      encoder,
    )
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
  Text(GPUxUITextPrimitive),
}

type UIMesh = IndexedMesh<u32, UIVertex, TriangleList>;

fn build_quad(
  device: &wgpu::Device,
  quad: &crate::Quad,
  color: Vec4<f32>,
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
    usage: wgpu::BufferUsage::VERTEX,
  });

  let index = bytemuck::cast_slice(index.as_slice());
  let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: index,
    usage: wgpu::BufferUsage::INDEX,
  });

  (index_buffer, vertex_buffer)
}

impl Primitive {
  pub fn create_gpu(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    renderer: &mut TextRenderer,
    res: &UIxGPUxResource,
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
            let (index_buffer, vertex_buffer) = build_quad(device, quad, Vec4::one());

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
        let text = renderer.create_gpu_text(
          device,
          encoder,
          Section {
            screen_position: (text.x, text.y),
            bounds: (text.max_width.unwrap_or(10000.), 10000.),
            text: vec![Text::new(text.content.as_str())
              .with_color([text.color.x, text.color.y, text.color.z, text.color.w])
              .with_scale(text.font_size)],
            ..Section::default()
          },
        );
        if let Some(text) = text {
          GPUxUIPrimitive::Text(text)
        } else {
          return None;
        }
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
  global_uniform_bind_group_layout: wgpu::BindGroupLayout,
  texture_bg_layout: wgpu::BindGroupLayout,
  sampler: wgpu::Sampler,
  global_bindgroup: wgpu::BindGroup,
}

impl WebGPUxUIRenderer {
  pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
    let global_ui_state = UIGlobalParameter {
      screen_size: Vec2::new(1000., 1000.),
    };

    let global_ui_state = UniformBufferData::create(device, global_ui_state.clone());
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
      global_uniform_bind_group_layout,
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
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    self.gpu_primitive_cache.clear();

    self.resource.global_ui_state.screen_size =
      Vec2::new(presentation.view_size.width, presentation.view_size.height);
    self.resource.global_ui_state.update(queue);

    self
      .text_renderer
      .resize_view(self.resource.global_ui_state.screen_size, queue);

    self.gpu_primitive_cache.extend(
      presentation
        .primitives
        .iter()
        .filter_map(|p| p.create_gpu(device, encoder, &mut self.text_renderer, &self.resource)),
    )
  }
}

#[derive(Debug, Copy, Clone)]
struct UIGlobalParameter {
  pub screen_size: Vec2<f32>,
}

unsafe impl bytemuck::Zeroable for UIGlobalParameter {}
unsafe impl bytemuck::Pod for UIGlobalParameter {}

impl UIGlobalParameter {
  fn get_shader_header() -> &'static str {
    "
    [[block]] struct UIGlobalParameter {
      screen_size: vec2<f32>;
    };
    [[group(0), binding(0)]] var ui_global_parameter: UIGlobalParameter;
    "
  }

  fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStage::VERTEX,
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

pub trait VertexBufferSourceType {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static>;
  fn get_shader_header() -> &'static str;
}

impl VertexBufferSourceType for Vec<UIVertex> {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<UIVertex>() as u64,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x2,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x2,
          offset: 4 * 2,
          shader_location: 1,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x4,
          offset: 4 * 2 + 4 * 2,
          shader_location: 2,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]] position: vec2<f32>,
      [[location(1)]] uv: vec2<f32>,
      [[location(2)]] color: vec4<f32>,
    "#
  }
}
