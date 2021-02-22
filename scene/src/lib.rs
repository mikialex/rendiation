pub mod scene;
pub use scene::*;

pub mod lights;
pub use lights::*;

pub mod materials;
pub use materials::*;

pub trait SceneRenderer {
  fn render(&mut self, scene: &mut Scene);
}
