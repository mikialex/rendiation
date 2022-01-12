pub mod directional;
pub use directional::*;

pub struct LightList<T> {
  pub lights: Vec<T>,
}
