use rendiation::geometry::quad_maker;
use rendiation::StandardGeometry;
use rendiation::{vertex, Vertex, WGPURenderer};
use rendiation_math::Vec4;

pub struct GUIRenderer {
  renderer: WGPURenderer,
  quad: StandardGeometry,
  view: Vec4<f32>,
}

impl GUIRenderer {
  pub fn new(renderer: WGPURenderer) -> Self {
    let quad = StandardGeometry::new_pair(quad_maker(), &renderer);
    GUIRenderer {
      renderer,
      quad,
      view: Vec4::new(0.0, 0.0, 100., 100.),
    }
  }

  pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {}
}
