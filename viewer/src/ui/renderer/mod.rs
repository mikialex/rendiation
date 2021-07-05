use rendiation_algebra::*;
use rendiation_renderable_mesh::{
  mesh::IndexedMesh,
  tessellation::{IndexedMeshTessellator, PlaneMeshParameter},
};

use crate::{
  renderer::{RenderPassCreator, Renderable, Renderer},
  scene::VertexBufferSourceType,
};

pub struct WebGPUxUIRenderPass<'a> {
  renderer: &'a mut WebGPUxUIRenderer,
}

pub struct UITextureCache {
  cached_target_frame: wgpu::TextureView,
  cached_target: wgpu::Texture,
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
          load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl<'r> Renderable for WebGPUxUIRenderPass<'r> {
  fn update(&mut self, renderer: &mut Renderer, encoder: &mut wgpu::CommandEncoder) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    let renderer = &self.renderer;
    renderer.gpu_primitive_cache.iter().for_each(|p| match p {
      GPUxUIPrimitive::SolidColor(p) => {
        pass.set_pipeline(&renderer.solid_color_pipeline);
        // pass.set_bind_group(0, &quad.bindgroup, &[])
        pass.set_index_buffer(p.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
        pass.draw_indexed(0..p.length, 0, 0..1);
      }
    })
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
}

pub struct WebGPUxUIRenderer {
  texture_cache: UITextureCache,
  gpu_primitive_cache: Vec<GPUxUIPrimitive>,
  solid_color_pipeline: wgpu::RenderPipeline,
  quad_mesh_instance: IndexedMesh,
}

impl WebGPUxUIRenderer {
  pub fn new(device: &wgpu::Device) -> Self {
    let quad_mesh_instance = PlaneMeshParameter::default().tessellate().mesh;
    todo!()
  }
}

struct UIGlobalParameter {
  pub screen_size: Vec2<f32>,
}

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

pub struct UIVertex {
  position: Vec2<f32>,
  uv: Vec2<f32>,
  color: Vec4<f32>,
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
      [[location(0)]] position: vec3<f32>,
      [[location(1)]] uv: vec3<f32>,
      [[location(2)]] color: vec2<f32>,
    "#
  }
}

fn create_solid_pipeline(
  device: &wgpu::Device,
  target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
  let global_uniform_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStage::VERTEX,
        ty: wgpu::BindingType::Buffer {
          has_dynamic_offset: false,
          min_binding_size: None,
          ty: wgpu::BufferBindingType::Uniform,
        },
        count: None,
      }],
    });

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
      entry_point: if cfg!(feature = "web") {
        "vs_web_main"
      } else {
        "vs_main"
      },
      module: &shader,
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: 5 * 4,
        step_mode: wgpu::InputStepMode::Vertex,
        // 0: vec2 position
        // 1: vec2 texture coordinates
        // 2: uint color
        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32],
      }],
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
