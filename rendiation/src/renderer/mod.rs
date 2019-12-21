use crate::renderer::shader_util::*;
use std::collections::HashMap;

pub mod r#const;
pub mod shader_util;

pub struct WGPURenderer {
  device: wgpu::Device,
  pipelines: HashMap<String, WGPUPipeline>,
}

pub struct WGPUPipeline {
  pipeline: wgpu::RenderPipeline,
  vertex_str: String,
  frag_str: String,
}

// impl WGPURenderer {
//   create
// }

pub struct WGPUPipelineDescriptorBuilder {
  vertex_shader: String,
  frag_shader: String,
  bindings: Vec<wgpu::BindGroupLayoutBinding>,
  
}

impl WGPUPipelineDescriptorBuilder {
  pub fn new() -> Self {
    WGPUPipelineDescriptorBuilder {
      vertex_shader: String::from(""),
      frag_shader:  String::from(""),
      bindings: Vec::new(),
    }
  }

  pub fn vertex_shader(&mut self, v: &str) -> &mut Self{
    self.vertex_shader = v.to_string();
    self
  }

  pub fn frag_shader(&mut self, v: &str) -> &mut Self{
    self.frag_shader = v.to_string();
    self
  }

  pub fn binding(&mut self, b: wgpu::BindGroupLayoutBinding) -> &mut Self {
    self.bindings.push(b);
    self
  }

  pub fn build(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    // Create pipeline layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      bindings: &self.bindings,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &[&bind_group_layout],
    });

     // Create the texture
     let size = 256u32;
     let texture_extent = wgpu::Extent3d {
         width: size,
         height: size,
         depth: 1,
     };
     let texture = device.create_texture(&wgpu::TextureDescriptor {
         size: texture_extent,
         array_layer_count: 1,
         mip_level_count: 1,
         sample_count: 1,
         dimension: wgpu::TextureDimension::D2,
         format: wgpu::TextureFormat::Rgba8UnormSrgb,
         usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
     });
     let texture_view = texture.create_default_view();

    // Create other resources
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: -100.0,
      lod_max_clamp: 100.0,
      compare_function: wgpu::CompareFunction::Always,
    });
    let mx_total = Self::generate_matrix(sc_desc.width as f32 / sc_desc.height as f32);
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let uniform_buf = device
      .create_buffer_mapped(16, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
      .fill_from_slice(mx_ref);

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &bind_group_layout,
      bindings: &[
        wgpu::Binding {
          binding: 0,
          resource: wgpu::BindingResource::Buffer {
            buffer: &uniform_buf,
            range: 0..64,
          },
        },
        wgpu::Binding {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&texture_view),
        },
        wgpu::Binding {
          binding: 2,
          resource: wgpu::BindingResource::Sampler(&sampler),
        },
      ],
    });

    // Create the render pipeline
    let vs_bytes = load_glsl(&self.vertex_shader, ShaderStage::Vertex);
    let fs_bytes = load_glsl(&self.frag_shader, ShaderStage::Fragment);
    let vs_module = device.create_shader_module(&vs_bytes);
    let fs_module = device.create_shader_module(&fs_bytes);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      layout: &pipeline_layout,
      vertex_stage: wgpu::ProgrammableStageDescriptor {
        module: &vs_module,
        entry_point: "main",
      },
      fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
        module: &fs_module,
        entry_point: "main",
      }),
      rasterization_state: Some(wgpu::RasterizationStateDescriptor {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: wgpu::CullMode::Back,
        depth_bias: 0,
        depth_bias_slope_scale: 0.0,
        depth_bias_clamp: 0.0,
      }),
      primitive_topology: wgpu::PrimitiveTopology::TriangleList,
      color_states: &[wgpu::ColorStateDescriptor {
        format: sc_desc.format,
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      }],
      depth_stencil_state: None,
      index_format: wgpu::IndexFormat::Uint16,
      vertex_buffers: &[wgpu::VertexBufferDescriptor {
        stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[
          wgpu::VertexAttributeDescriptor {
            format: wgpu::VertexFormat::Float4,
            offset: 0,
            shader_location: 0,
          },
          wgpu::VertexAttributeDescriptor {
            format: wgpu::VertexFormat::Float2,
            offset: 4 * 4,
            shader_location: 1,
          },
        ],
      }],
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
    })
  }
}

// use rendiation_render_entity::*;
// impl Shading<WGPURenderer> for DynamicShading {
//   fn get_index(&self) -> usize {

//   }
//   fn get_vertex_str(&self) -> &str {}
//   fn get_fragment_str(&self) -> &str {}
//   fn make_gpu_port(&self, renderer: &WGPURenderer) -> Rc<dyn ShadingGPUPort<WGPURenderer>> {}
// }

pub struct BlockShading {}
