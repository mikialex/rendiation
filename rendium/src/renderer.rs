use rendiation::geometry::quad_maker;
use rendiation::StandardGeometry;
use rendiation::{vertex, Vertex, WGPURenderer};
use rendiation_math::Vec4;

pub struct GUIRenderer {
  renderer: WGPURenderer,
  quad: StandardGeometry,
  view_port: Vec4<f32>,
}

impl GUIRenderer {
  pub fn new(renderer: WGPURenderer) -> Self {
    let quad = StandardGeometry::new_pair(quad_maker(), &renderer);
    GUIRenderer {
      renderer,
      quad,
      view_port: Vec4::new(0.0, 0.0, 100., 100.),
    }
  }

  pub fn draw_rect(&mut self) {}
}
