use rendiation::StandardGeometry;
use rendiation::{vertex, WGPURenderer, Vertex};

pub struct GUIRenderer {
  renderer: WGPURenderer,
  quad: StandardGeometry,
}

fn quad_maker() -> (Vec<Vertex>, Vec<u16>){
  let data = [
    vertex([-1.0,-1.0,0.0], [-1.0,-1.0,1.0], [0.0, 0.0]),
    vertex([-1.0,1.0,0.0], [-1.0,-1.0,1.0], [0.0, 0.0]),
    vertex([1.0,1.0,0.0], [-1.0,-1.0,1.0], [0.0, 0.0]),
    vertex([1.0,-1.0,0.0], [-1.0,-1.0,1.0], [0.0, 0.0]),
  ];
  let index = [0, 2, 1, 2, 0, 3];
  (data.to_vec(), index.to_vec())
}

impl GUIRenderer {
  pub fn new(renderer: WGPURenderer) -> Self {
    todo!();
    
  }

  pub fn draw_rect(&mut self) {}
}
