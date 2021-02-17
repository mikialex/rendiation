use crate::element::quad::QuadLayout;
// use crate::renderer::shader::CopyShading;
// use crate::renderer::shader::CopyShadingParam;
// use crate::renderer::shader::QuadShading;
use rendiation_algebra::{Mat4, Vec4};
use rendiation_render_entity::*;
use rendiation_webgpu::*;

mod shader;
use render_target::{RenderTarget, RenderTargetAble, TargetInfoProvider};
use rendiation_renderable_mesh::{tessellation::*, wgpu::GPUGeometry};
pub use shader::*;

pub struct GUIRenderer {
  quad: GPUGeometry,
  view: Vec4<f32>,
  projection: OrthographicProjection,
  camera: Camera,
  camera_gpu_buffer: WGPUBuffer,
  canvas: RenderTarget,
  // quad_pipeline: QuadShading,
  // copy_screen_sampler: WGPUSampler,
  // copy_screen_pipeline: CopyShading,
}

impl GUIRenderer {
  pub fn new(
    renderer: &mut WGPURenderer,
    size: (f32, f32),
    screen_target: &impl TargetInfoProvider,
  ) -> Self {
    let view = Vec4::new(0.0, 0.0, size.0, size.1);
    let mut quad = GPUGeometry::from(Quad.tessellate(&()));
    quad.update_gpu(renderer);
    let canvas = WGPUTexture::new_as_target_default(&renderer, (size.0 as usize, size.1 as usize));
    let canvas = RenderTarget::from_one_texture(canvas);
    // let canvas = //

    let projection = OrthographicProjection::default();
    let camera = Camera::new();

    let mx_total = OPENGL_TO_WGPU_MATRIX * camera.get_vp_matrix();
    let mx_ref: &[u8] = mx_total.as_ref();
    let camera_gpu_buffer = WGPUBuffer::new(
      renderer,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    // let quad_pipeline = QuadShading::new(renderer, &canvas);
    // let copy_screen_pipeline = CopyShading::new(renderer, screen_target);
    let copy_screen_sampler = WGPUSampler::default(renderer);
    let mut renderer = GUIRenderer {
      quad,
      view,
      projection,
      camera,
      camera_gpu_buffer,
      canvas,
      // quad_pipeline,
      // copy_screen_pipeline,
      // copy_screen_sampler,
    };
    renderer.update_camera();
    renderer
  }

  fn update_camera(&mut self) {
    let orth = &mut self.projection;
    orth.top = 0.;
    orth.left = 0.;
    orth.bottom = -self.view.w;
    orth.right = -self.view.z;
    orth.near = -1.;
    orth.far = 1.;
    self.camera.update_by(&self.projection);
  }

  pub fn resize(&mut self, size: (f32, f32), renderer: &WGPURenderer) {
    self.view.z = size.0;
    self.view.w = size.1;
    self
      .canvas
      .resize(&renderer, (size.0 as usize, size.1 as usize));
    self.update_camera();
  }

  pub fn clear_canvas(&self, renderer: &mut WGPURenderer) {
    self
      .canvas
      .create_render_pass_builder()
      .first_color(|c| c.load_with_clear((1., 1., 1.).into(), 0.5).ok())
      .create(renderer);
  }

  pub fn update_to_screen(&mut self, renderer: &mut WGPURenderer, screen: &impl RenderTargetAble) {
    // let bindgroup = CopyShadingParam {
    //   texture_view: self.canvas.get_first_color_attachment().view(),
    //   sampler: &self.copy_screen_sampler,
    // }
    // .create_bindgroup(renderer);

    // let mut pass = screen
    //   .create_render_pass_builder()
    //   .create(&mut renderer.encoder);

    // pass
    //   .gpu_pass
    //   .set_pipeline(&self.copy_screen_pipeline.pipeline.pipeline);
    // pass
    //   .gpu_pass
    //   .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    // // self.quad.render(&mut pass); // todo
  }

  pub fn draw_rect(
    &mut self,
    renderer: &mut WGPURenderer,
    quad_layout: &QuadLayout,
    color: &Vec4<f32>,
  ) {
    // let mvp = quad_layout.compute_matrix(&self.camera);
    // let mx_ref: &[f32; 16] = mvp.as_ref();
    // self.camera_gpu_buffer.update(renderer, mx_ref);

    // let color_ref: &[f32; 4] = color.as_ref();
    // let color_uniform = WGPUBuffer::new(
    //   renderer,
    //   color_ref,
    //   wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    // );

    // let bindgroup = QuadShadingParam {
    //   transform: &self.camera_gpu_buffer,
    //   color: &color_uniform,
    // }
    // .create_bindgroup(renderer);

    // let mut pass = self
    //   .canvas
    //   .create_render_pass_builder()
    //   .create(&mut renderer.encoder);

    // pass
    //   .gpu_pass
    //   .set_pipeline(&self.quad_pipeline.pipeline.pipeline);
    // pass
    //   .gpu_pass
    //   .set_bind_group(0, &bindgroup.gpu_bindgroup, &[]);

    // self.quad.render(&mut pass);
  }
}
