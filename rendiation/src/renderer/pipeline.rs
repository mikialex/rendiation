use core::marker::PhantomData;
use crate::renderer::texture::WGPUTexture;
use crate::{WGPUBindGroup, WGPURenderer};

pub struct WGPUPipeline {
  pub pipeline: wgpu::RenderPipeline,
}

pub trait VertexProvider {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
}
pub trait GeometryProvider {
  fn get_geometry_layout_descriptor() -> Vec<wgpu::VertexBufferDescriptor<'static>>;
  fn get_index_format() -> wgpu::IndexFormat;
}

pub struct ShaderParamGPU<T> {
  pub bindgroup: WGPUBindGroup,
  phantom: PhantomData<T>,
}
impl<T> ShaderParamGPU<T>{
  pub fn new(bindgroup: WGPUBindGroup) -> Self {
    ShaderParamGPU {
      bindgroup,
      phantom: PhantomData::default(),
    }
  }
}

pub trait BindGroupProvider: Sized {
  fn provide_layout(renderer: &WGPURenderer) -> &'static wgpu::BindGroupLayout;
  fn create_bindgroup(&mut self, renderer: &WGPURenderer) -> ShaderParamGPU<Self>;
}

pub struct StaticPipelineBuilder<'a> {
  renderer: &'a WGPURenderer,
  vertex_shader: &'static str,
  frag_shader: &'static str,
  bindgroup_layouts: Vec<&'static wgpu::BindGroupLayout>,
  vertex_layouts: Vec<wgpu::VertexBufferDescriptor<'static>>,
  index_format: wgpu::IndexFormat,
  pub depth_format: Option<wgpu::TextureFormat>,
  pub color_target_format: wgpu::TextureFormat,
}

impl<'a> StaticPipelineBuilder<'a> {
  pub fn new(
    renderer: &'a WGPURenderer,
    vertex_shader: &'static str,
    frag_shader: &'static str,
  ) -> Self {
    Self {
      renderer,
      vertex_shader,
      frag_shader,
      bindgroup_layouts: Vec::new(),
      vertex_layouts: Vec::new(),
      index_format: wgpu::IndexFormat::Uint16,
      depth_format: None,
      color_target_format: wgpu::TextureFormat::Rgba8UnormSrgb,
    }
  }

  pub fn binding_group<T: BindGroupProvider>(&mut self) -> &mut Self {
    self
      .bindgroup_layouts
      .push(T::provide_layout(self.renderer));
    self
  }

  pub fn geometry<T: GeometryProvider>(&mut self) -> &mut Self {
    self
      .vertex_layouts
      .extend(T::get_geometry_layout_descriptor());
    self.index_format = T::get_index_format();
    self
  }

  pub fn vertex<T: VertexProvider>(&mut self) -> &mut Self {
    self.vertex_layouts.push(T::get_buffer_layout_descriptor());
    self
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

  pub fn build(&self) -> WGPUPipeline {
    let device = &self.renderer.device;
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &self.bindgroup_layouts,
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
       index_format: self.index_format,
       vertex_buffers: &self.vertex_layouts,
       sample_count: 1,
       sample_mask: !0,
       alpha_to_coverage_enabled: false,
     };
 
     let pipeline = device.create_render_pipeline(&pipeline_des);
 
     WGPUPipeline {
       pipeline,
     }
  }
}
