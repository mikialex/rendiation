use rendiation::geometry::quad_maker;
use rendiation::*;
use rendiation_math::Vec4;
use rendiation_render_entity::*;

pub struct GUIRenderer {
  quad: StandardGeometry,
  view: Vec4<f32>,
  camera: OrthographicCamera,
  camera_gpu_buffer: WGPUBuffer,
  canvas: WGPUTexture,
  quad_pipeline: WGPUPipeline,
  copy_screen_sampler: WGPUSampler,
  copy_screen_pipeline: WGPUPipeline,
}

impl GUIRenderer {
  pub fn new(renderer: &WGPURenderer, size: (f32, f32)) -> Self {
    let quad = StandardGeometry::new_pair(quad_maker(), &renderer);
    let canvas = WGPUTexture::new_as_target(&renderer.device, (size.0 as u32, size.1 as u32));

    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./quad.vert"))
      .frag_shader(include_str!("./quad.frag"))
      .binding_group(BindGroupLayoutBuilder::new().bind_uniform_buffer(ShaderStage::Vertex))
      .to_color_target(&canvas);

    let quad_pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);

    let camera = OrthographicCamera::new();
    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let camera_gpu_buffer = WGPUBuffer::new(
      &renderer.device,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./copy.vert"))
      .frag_shader(include_str!("./copy.frag"))
      .binding_group(
        BindGroupLayoutBuilder::new()
          // .bind_uniform_buffer(ShaderStage::Vertex)
          .bind_texture2d(ShaderStage::Fragment)
          .bind_sampler(ShaderStage::Fragment),
      )
      .to_screen_target(&renderer);

    let copy_screen_pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);
    let copy_screen_sampler = WGPUSampler::new(&renderer.device);
    GUIRenderer {
      quad,
      view: Vec4::new(0.0, 0.0, size.0, size.1),
      camera,
      camera_gpu_buffer,
      canvas,
      quad_pipeline,
      copy_screen_pipeline,
      copy_screen_sampler,
    }
  }

  pub fn update_to_screen(&self, renderer: &mut WGPURenderer, screen_view: &wgpu::TextureView) {
    let bindgroup = BindGroupBuilder::new()
      .texture(self.canvas.view())
      .sampler(&self.copy_screen_sampler)
      .build(
        &renderer.device,
        &self.copy_screen_pipeline.get_bindgroup_layout(0),
      );

    {
      let mut pass = WGPURenderPass::build()
        // .output(self.canvas.view())
        .output_with_clear(self.canvas.view(), (1., 1., 1., 0.))
        .create(&mut renderer.encoder);
    }

    let mut pass = WGPURenderPass::build()
      .output(screen_view)
      .create(&mut renderer.encoder);

    pass
      .gpu_pass
      .set_pipeline(&self.copy_screen_pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    self.quad.render(&mut pass);
  }

  pub fn draw_rect(
    &mut self,
    renderer: &mut WGPURenderer,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
  ) {
    let bindgroup = BindGroupBuilder::new()
      .buffer(&self.camera_gpu_buffer)
      .build(
        &renderer.device,
        &self.quad_pipeline.get_bindgroup_layout(0),
      );

    let mut pass = WGPURenderPass::build()
      // .output(self.canvas.view())
      .output_with_clear(self.canvas.view(), (1., 1., 1., 1.))
      .create(&mut renderer.encoder);

    pass.gpu_pass.set_pipeline(&self.quad_pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    // self.quad.render(&mut pass);
  }
}
