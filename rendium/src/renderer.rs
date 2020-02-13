use rendiation::StandardGeometry;
use rendiation::{vertex, Vertex, WGPURenderer};
use rendiation_math::Vec4;

pub struct GUIRenderer {
  renderer: WGPURenderer,
  quad: StandardGeometry,
  view_port: Vec4<f32>,
}

fn quad_maker() -> (Vec<Vertex>, Vec<u16>) {
  let data = [
    vertex([-1.0, -1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 0.0]),
    vertex([-1.0, 1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 0.0]),
    vertex([1.0, 1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 0.0]),
    vertex([1.0, -1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 0.0]),
  ];
  let index = [0, 2, 1, 2, 0, 3];
  (data.to_vec(), index.to_vec())
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
