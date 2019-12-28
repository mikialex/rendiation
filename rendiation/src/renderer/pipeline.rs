pub struct WGPUPipeline {
  pipeline: wgpu::RenderPipeline,
  bind_groups: Vec<wgpu::BindGroup>,
}

pub trait VertexProvider {
  fn get_stride() -> usize;
}

pub struct WGPUPipelineDescriptorBuilder {
  vertex_shader: String,
  frag_shader: String,
  bindings: Vec<wgpu::BindGroupLayoutBinding>,
}

impl WGPUPipelineDescriptorBuilder {
  pub fn new() -> Self {
    WGPUPipelineDescriptorBuilder {
      vertex_shader: String::from(""),
      frag_shader: String::from(""),
      bindings: Vec::new(),
    }
  }

  pub fn vertex_shader(&mut self, v: &str) -> &mut Self {
    self.vertex_shader = v.to_string();
    self
  }

  pub fn frag_shader(&mut self, v: &str) -> &mut Self {
    self.frag_shader = v.to_string();
    self
  }

  pub fn binding(&mut self, b: wgpu::BindGroupLayoutBinding) -> &mut Self {
    self.bindings.push(b);
    self
  }

  // pub fn use_buffer(&mut )

  pub fn build<T: VertexProvider>(&self, device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) -> WGPUPipeline {
    // Create pipeline layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      bindings: &self.bindings,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &[&bind_group_layout],
    });

    // Create the render pipeline
    use crate::renderer::shader_util::*;
    let vs_bytes = load_glsl(&self.vertex_shader, ShaderStage::Vertex);
    let fs_bytes = load_glsl(&self.frag_shader, ShaderStage::Fragment);
    let vs_module = device.create_shader_module(&vs_bytes);
    let fs_module = device.create_shader_module(&fs_bytes);

    let pipeline_des = wgpu::RenderPipelineDescriptor {
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
        stride: T::get_stride() as wgpu::BufferAddress,
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
    };

    let pipeline = device.create_render_pipeline(&pipeline_des);

    WGPUPipeline {
      pipeline,
      bind_groups: Vec::new(),
    }
  }
}
