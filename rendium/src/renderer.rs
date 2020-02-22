use rendiation::geometry::quad_maker;
use rendiation::*;
use rendiation_render_entity::*;
use rendiation_math::Vec4;

pub struct GUIRenderer {
  quad: StandardGeometry,
  view: Vec4<f32>,
  camera: OrthographicCamera,
  canvas: WGPUTexture,
  quad_pipeline: WGPUPipeline,
}

impl GUIRenderer {
  pub fn new(renderer: &WGPURenderer, size: (f32, f32)) -> Self {
    let quad = StandardGeometry::new_pair(quad_maker(), &renderer);
    let canvas = WGPUTexture::new_as_target(&renderer.device, (size.0 as u32, size.1 as u32));

    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./quad.vert"))
      .frag_shader(include_str!("./quad.frag"))
      .binding_group(
        BindGroupLayoutBuilder::new()
          .bind_uniform_buffer(ShaderStage::Vertex)
          .bind_texture2d(ShaderStage::Fragment)
          .bind_sampler(ShaderStage::Fragment)
      )
      .with_swapchain_target(&renderer.swap_chain.swap_chain_descriptor);

    let quad_pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);

    GUIRenderer {
      quad,
      view: Vec4::new(0.0, 0.0, size.0, size.1),
      camera: OrthographicCamera::new(),
      canvas,
      quad_pipeline,
    }
  }

  pub fn draw_rect(&mut self, renderer: &mut WGPURenderer,x: f32, y: f32, width: f32, height: f32) {
    let mut pass = WGPURenderPass::build().create(&mut renderer.encoder);
    
  }
}
