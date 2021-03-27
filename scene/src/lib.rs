pub mod scene;
pub use scene::*;

pub mod materials;
pub use materials::*;

pub trait SceneRenderer {
  fn render(&mut self, scene: &mut Scene);
}
