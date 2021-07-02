use rendiation_algebra::*;

#[derive(Debug, Clone)]
pub enum Primitive {
  Quad(Quad),
  Text(TextInfo),
}

#[derive(Debug, Clone)]
pub struct Quad {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

#[derive(Debug, Clone)]
pub struct TextInfo {
  content: String,
  max_width: Option<f32>,
  x: f32,
  y: f32,
}

impl Primitive {
  pub fn test_pointer_in(&self, pointer: Vec2<f32>) -> bool {
    match self {
      Primitive::Quad(_) => todo!(),
      Primitive::Text(_) => todo!(),
    }
  }
}
