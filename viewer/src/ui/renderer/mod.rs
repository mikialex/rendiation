use glyph_brush::{Section, Text};
use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{IndexedMesh, TriangleList};
use rendiation_webgpu::*;
use wgpu::util::DeviceExt;

mod text;

use crate::scene::VertexBufferSourceType;

use self::text::{GPUxUITextPrimitive, TextRenderer};

use super::{Primitive, UIPresentation};

pub struct WebGPUxUIRenderPass<'a> {
  pub renderer: &'a mut WebGPUxUIRenderer,
  pub presentation: &'a UIPresentation,
}

pub struct UITextureCache {
  // cached_target_frame: wgpu::TextureView,
// cached_target: wgpu::Texture,
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
        pass.set_pipeline(&renderer.solid_color_pipeline);
        pass.set_bind_group(0, &self.renderer.global_bindgroup, &[]);
        pass.set_index_buffer(p.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
      GPUxUIPrimitive::Text(text) => {
        self.renderer.text_renderer.draw_gpu_text(pass, text);
      }
    });
  }

  fn update(&mut self, renderer: &mut GPU, encoder: &mut wgpu::CommandEncoder) {
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

pub enum GPUxUIPrimitive {
  SolidColor(GPUxUISolidColorPrimitive),
  Text(GPUxUITextPrimitive),
}

type UIMesh = IndexedMesh<u32, UIVertex, TriangleList>;

impl Primitive {
  pub fn create_gpu(
    &self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    renderer: &mut TextRenderer,
  ) -> Option<GPUxUIPrimitive> {
    let p = match self {
      #[rustfmt::skip]
      Primitive::Quad(quad) => {
        let mut vertices = Vec::new();
        vertices.push(vertex((quad.x, quad.y), (0., 0.), (1., 1., 1., 1.)));
        vertices.push(vertex((quad.x, quad.y + quad.height), (0., 0.), (1., 1., 1., 1.)));
        vertices.push(vertex((quad.x + quad.width, quad.y), (0., 0.), (1., 1., 1., 1.)));
        vertices.push(vertex((quad.x + quad.width, quad.y + quad.height), (0., 0.), (1., 1., 1., 1.)));
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

        // let index_mesh: UIMesh = IndexedMesh::new(vertices, index);

        GPUxUIPrimitive::SolidColor(GPUxUISolidColorPrimitive {
          vertex_buffer,
          index_buffer,
          length: 6,
        })
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
  texture_cache: UITextureCache,
  gpu_primitive_cache: Vec<GPUxUIPrimitive>,
  solid_color_pipeline: wgpu::RenderPipeline,
  global_ui_state: UniformBufferData<UIGlobalParameter>,
  global_uniform_bind_group_layout: wgpu::BindGroupLayout,
  global_bindgroup: wgpu::BindGroup,
  text_renderer: TextRenderer,
}

impl WebGPUxUIRenderer {
  pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
    let texture_cache = UITextureCache {};
    let global_ui_state = UIGlobalParameter {
      screen_size: Vec2::new(1000., 1000.),
    };

    let global_ui_state = UniformBufferData::create(device, global_ui_state.clone());
    let global_uniform_bind_group_layout =
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
      });
    let global_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &global_uniform_bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: global_ui_state.as_bindable(),
      }],
      label: None,
    });

    let solid_color_pipeline =
      create_solid_pipeline(device, target_format, &global_uniform_bind_group_layout);

    let text_renderer = TextRenderer::new(
      device,
      wgpu::FilterMode::Linear,
      wgpu::TextureFormat::Bgra8UnormSrgb,
    );

    Self {
      texture_cache,
      gpu_primitive_cache: Vec::new(),
      solid_color_pipeline,
      global_ui_state,
      global_uniform_bind_group_layout,
      global_bindgroup,
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

    self.global_ui_state.screen_size =
      Vec2::new(presentation.view_size.x, presentation.view_size.y);
    self.global_ui_state.update(queue);

    self
      .text_renderer
      .resize_view(self.global_ui_state.screen_size, queue);

    self.gpu_primitive_cache.extend(
      presentation
        .primitives
        .iter()
        .filter_map(|p| p.create_gpu(device, encoder, &mut self.text_renderer)),
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

fn create_solid_pipeline(
  device: &wgpu::Device,
  target_format: wgpu::TextureFormat,
  global_uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("ui_solid_pipeline_layout"),
    bind_group_layouts: &[&global_uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

  let shader_source = format!(
    "
      {global_header}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] color: vec4<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        {vertex_header}
      ) -> VertexOutput {{
        var out: VertexOutput;

        out.color = color;

        out.position = vec4<f32>(
            2.0 * position.x / ui_global_parameter.screen_size.x - 1.0,
            1.0 - 2.0 * position.y / ui_global_parameter.screen_size.y,
            0.0,
            1.0,
        );

        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return in.color;
      }}
      
      ",
    vertex_header = Vec::<UIVertex>::get_shader_header(),
    global_header = UIGlobalParameter::get_shader_header(),
  );

  let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source.as_str())),
    flags: wgpu::ShaderFlags::all(),
  });

  let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("ui_solid_pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      entry_point: "vs_main",
      module: &shader,
      buffers: &[Vec::<UIVertex>::vertex_layout()],
    },
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleList,
      clamp_depth: false,
      conservative: false,
      cull_mode: None,
      front_face: wgpu::FrontFace::default(),
      polygon_mode: wgpu::PolygonMode::default(),
      strip_index_format: None,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState {
      alpha_to_coverage_enabled: false,
      count: 1,
      mask: !0,
    },

    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[wgpu::ColorTargetState {
        format: target_format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrite::ALL,
      }],
    }),
  });

  render_pipeline
}
