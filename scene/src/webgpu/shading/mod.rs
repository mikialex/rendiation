pub mod physical;
pub use physical::*;

pub trait LightableSurfaceShading {
  fn struct_define() -> &'static str;
  fn struct_construct() -> &'static str;
}
