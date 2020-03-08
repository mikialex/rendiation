use crate::element::quad::QuadLayout;
use crate::renderer::shader::CopyShading;
use crate::renderer::shader::CopyShadingParam;
use crate::renderer::shader::QuadShading;
use rendiation::geometry::quad_maker;
use rendiation::*;
use rendiation_math::{Mat4, Vec4};
use rendiation_render_entity::*;

mod shader;
pub use shader::*;

pub struct GUIRenderer {
  quad: StandardGeometry,
  view: Vec4<f32>,
  camera: OrthographicCamera,
  camera_gpu_buffer: WGPUBuffer,
  canvas: WGPUTexture,
  quad_pipeline: QuadShading,
  copy_screen_sampler: WGPUSampler,
  copy_screen_pipeline: CopyShading,
}

impl GUIRenderer {
  pub fn new(renderer: &mut WGPURenderer, size: (f32, f32)) -> Self {
    let view = Vec4::new(0.0, 0.0, size.0, size.1);
    let mut quad = StandardGeometry::from(quad_maker());
    quad.update_gpu(renderer);
    let canvas = WGPUTexture::new_as_target(&renderer.device, (size.0 as u32, size.1 as u32));

    let camera = OrthographicCamera::new();

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    let camera_gpu_buffer = WGPUBuffer::new(
      &renderer.device,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let quad_pipeline = QuadShading::new(renderer, &canvas);
    let copy_screen_pipeline = CopyShading::new(renderer);
    let copy_screen_sampler = WGPUSampler::new(&renderer.device);
    let mut renderer = GUIRenderer {
      quad,
      view,
      camera,
      camera_gpu_buffer,
      canvas,
      quad_pipeline,
      copy_screen_pipeline,
      copy_screen_sampler,
    };
    renderer.update_camera();
    renderer
  }

  fn update_camera(&mut self){
    let camera = &mut self.camera;
    camera.top = 0.;
    camera.left = 0.;
    camera.bottom = -self.view.w;
    camera.right = -self.view.z;
    camera.near = -1.;
    camera.far= 1.;
    camera.update_projection();
  }

  pub fn resize(&mut self, size: (f32, f32), renderer: &WGPURenderer) {
    self.view.z = size.0;
    self.view.w = size.1;
    self.canvas.resize(&renderer.device, (size.0 as usize, size.1 as usize));
    self.update_camera();
  }

  pub fn clear_canvas(&self, renderer: &mut WGPURenderer){
      WGPURenderPass::build()
        .output_with_clear(self.canvas.view(), (1., 1., 1., 0.5))
        .create(&mut renderer.encoder);
  }

  pub fn update_to_screen(&mut self, renderer: &mut WGPURenderer, screen_view: &wgpu::TextureView) {
    let bindgroup = CopyShadingParam {
      texture_view: self.canvas.view(),
      sampler: &self.copy_screen_sampler,
    }
    .create_bindgroup(renderer);

    let mut pass = WGPURenderPass::build()
      .output(screen_view)
      .create(&mut renderer.encoder);

    pass
      .gpu_pass
      .set_pipeline(&self.copy_screen_pipeline.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    self.quad.render(&mut pass);
  }

  pub fn draw_rect(
    &mut self,
    renderer: &mut WGPURenderer,
    quad_layout: &QuadLayout,
  ) {
    let mvp = quad_layout.compute_matrix(&self.camera);
    let mx_ref: &[f32; 16] = mvp.as_ref();
    self.camera_gpu_buffer.update(&renderer.device, &mut renderer.encoder, mx_ref);

    let color = Vec4::new(1.0, 0.0, 0.0, 0.5);
    let color_ref: &[f32; 4] = color.as_ref();
    let color_uniform = WGPUBuffer::new(
      &renderer.device,
      color_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let bindgroup = QuadShadingParam {
      transform: &self.camera_gpu_buffer,
      color: &color_uniform,
    }
    .create_bindgroup(renderer);

    let mut pass = WGPURenderPass::build()
      .output(self.canvas.view())
      .create(&mut renderer.encoder);

    pass
      .gpu_pass
      .set_pipeline(&self.quad_pipeline.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    self.quad.render(&mut pass);
  }
}
