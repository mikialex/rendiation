use rendiation::geometry::quad_maker;
use rendiation::*;
use rendiation_render_entity::*;
use rendiation_math::Vec4;

pub struct GUIRenderer {
  quad: StandardGeometry,
  view: Vec4<f32>,
  camera: OrthographicCamera,
  canvas: WGPUTexture,
}

impl GUIRenderer {
  pub fn new(renderer: &WGPURenderer, size: (f32, f32)) -> Self {
    let quad = StandardGeometry::new_pair(quad_maker(), &renderer);
    let canvas = WGPUTexture::new_as_target(&renderer.device, (size.0 as u32, size.1 as u32));
    GUIRenderer {
      quad,
      view: Vec4::new(0.0, 0.0, size.0, size.1),
      camera: OrthographicCamera::new(),
      canvas
    }
  }

  pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {}
}
