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

fn computeQuadMatrix( 
  camera: &OrthographicCamera,
  x: f32,
  y: f32,
  width: f32,
  height: f32,){

}

impl GUIRenderer {
  pub fn new(renderer: &mut WGPURenderer, size: (f32, f32)) -> Self {
    let mut quad = StandardGeometry::from(quad_maker());
    quad.update_gpu(renderer);
    let canvas = WGPUTexture::new_as_target(&renderer.device, (size.0 as u32, size.1 as u32));

    let mut camera = OrthographicCamera::new();
    camera.top = 0.;
    camera.left = 0.;
    camera.bottom = -1000.;
    camera.right = -1000.;
    camera.near = -1.;
    camera.far= 1.;

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
    GUIRenderer {
      quad,
      view: Vec4::new(0.0, 0.0, 1000., 1000.),
      camera,
      camera_gpu_buffer,
      canvas,
      quad_pipeline,
      copy_screen_pipeline,
      copy_screen_sampler,
    }
  }

  pub fn update_to_screen(&self, renderer: &mut WGPURenderer, screen_view: &wgpu::TextureView) {
    let bindgroup = CopyShadingParam {
      texture_view: self.canvas.view(),
      sampler: &self.copy_screen_sampler,
    }
    .create_bindgroup(renderer);

    // {
    //   WGPURenderPass::build()
    //     // .output(self.canvas.view())
    //     .output_with_clear(self.canvas.view(), (1., 1., 1., 0.5))
    //     .create(&mut renderer.encoder);
    // }

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
    x: f32,
    y: f32,
    width: f32,
    height: f32,
  ) {
    let model_mat = Mat4::new(
      width, 0.0, 0.0, 0.0, 
			0.0, height, 0.0, 0.0, 
			0.0, 0.0, 1.0, 0.0, 
			x, y, 0.0, 1.0
    );
    let mvp = self.camera.get_vp_matrix() * model_mat;
    let mx_ref: &[f32; 16] = mvp.as_ref();
    self.camera_gpu_buffer.update(&renderer.device, &mut renderer.encoder, mx_ref);
    let bindgroup = QuadShadingParam {
      buffer: &self.camera_gpu_buffer,
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
