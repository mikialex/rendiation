pub mod background;
pub mod camera;
pub mod node;
pub mod scene;

pub use background::*;
pub use camera::*;
pub use node::*;
pub use scene::*;

pub mod materials;
pub use materials::*;

pub trait SceneRenderer {
  fn render(&mut self, scene: &mut Scene);
}
