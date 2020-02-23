use crate::renderer::WGPURenderer;
use crate::renderer::bindgroup_layout::BindGroupLayoutBuilder;
use crate::WGPUTexture;

pub trait GeometryProvider<'a> {
  fn get_geometry_layout_descriptor() -> Vec<wgpu::VertexBufferDescriptor<'a>>;
  fn get_index_format() -> wgpu::IndexFormat;
}

/// impl your custom vertex data layout
pub trait VertexProvider<'a> {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'a>;
}

pub struct WGPUPipeline {
  pub pipeline: wgpu::RenderPipeline,
  bind_group_layouts: Vec<wgpu::BindGroupLayout>,
}

impl WGPUPipeline {
  pub fn get_bindgroup_layout(&self, index: usize) -> &wgpu::BindGroupLayout{
    &self.bind_group_layouts[index]
  }
}

pub struct WGPUPipelineDescriptorBuilder {
  vertex_shader: String,
  frag_shader: String,
  binding_groups: Vec<BindGroupLayoutBuilder>,
  pub depth_format: Option<wgpu::TextureFormat>,
  pub color_target_format: wgpu::TextureFormat,
}

impl<'a> WGPUPipelineDescriptorBuilder {
  pub fn new() -> Self {
    WGPUPipelineDescriptorBuilder {
      vertex_shader: String::from(""),
      frag_shader: String::from(""),
      binding_groups: Vec::new(),
      depth_format: None,
      color_target_format: wgpu::TextureFormat::Rgba8UnormSrgb,
    }
  }

  pub fn with_depth_stencil(&mut self, target: &WGPUTexture) -> &mut Self {
    self.depth_format = Some(*target.format());
    self
  }

  pub fn to_color_target(&mut self, target: &WGPUTexture) -> &mut Self {
    self.color_target_format = *target.format();
    self
  }

  pub fn to_screen_target(&mut self, renderer: &WGPURenderer) -> &mut Self {
    self.color_target_format = renderer.swap_chain_format;
    self
  }

  pub fn vertex_shader(&mut self, v: &str) -> &mut Self {
    self.vertex_shader = v.to_string();
    self
  }

  pub fn frag_shader(&mut self, v: &str) -> &mut Self {
    self.frag_shader = v.to_string();
    self
  }

  pub fn binding_group(&mut self, b: BindGroupLayoutBuilder) -> &mut Self {
    self.binding_groups.push(b);
    self
  }

  pub fn build<T: GeometryProvider<'a>>(&self, device: &wgpu::Device) -> WGPUPipeline {
    let bind_group_layouts: Vec<_> = self
      .binding_groups
      .iter()
      .map(|builder| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          bindings: &builder.bindings,
        })
      })
      .collect();

    let bind_group_layouts_ref: Vec<&wgpu::BindGroupLayout> = bind_group_layouts
      .iter()
      .map(|l| {
        let l: &wgpu::BindGroupLayout = l;
        l
      })
      .collect();

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &bind_group_layouts_ref,
    });

    // Create the render pipeline
    use crate::renderer::shader_util::*;
    let vs_bytes = load_glsl(&self.vertex_shader, ShaderStage::Vertex);
    let fs_bytes = load_glsl(&self.frag_shader, ShaderStage::Fragment);
    let vs_module = device.create_shader_module(&vs_bytes);
    let fs_module = device.create_shader_module(&fs_bytes);

    let depth_stencil_state = self.depth_format.map(|format|{
      wgpu::DepthStencilStateDescriptor {
        format,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
        stencil_read_mask: 0,
        stencil_write_mask: 0,
      }
    });

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
        cull_mode: wgpu::CullMode::None,
        depth_bias: 0,
        depth_bias_slope_scale: 0.0,
        depth_bias_clamp: 0.0,
      }),
      primitive_topology: wgpu::PrimitiveTopology::TriangleList,
      color_states: &[wgpu::ColorStateDescriptor {
        format: self.color_target_format,
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
      }],
      depth_stencil_state,
      index_format: wgpu::IndexFormat::Uint16,
      vertex_buffers: &T::get_geometry_layout_descriptor(),
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
    };

    let pipeline = device.create_render_pipeline(&pipeline_des);

    WGPUPipeline {
      pipeline,
      bind_group_layouts,
    }
  }
}
