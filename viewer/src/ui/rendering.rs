use rendiation_algebra::*;

#[derive(Debug, Clone)]
pub enum Primitive {
  Quad(Quad),
  Text(TextInfo),
}

#[derive(Debug, Clone)]
pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

#[derive(Debug, Clone)]
pub struct TextInfo {
  pub content: String,
  pub max_width: Option<f32>,
  pub x: f32,
  pub y: f32,
}

impl Primitive {
  pub fn test_pointer_in(&self, pointer: Vec2<f32>) -> bool {
    match self {
      Primitive::Quad(_) => todo!(),
      Primitive::Text(_) => todo!(),
    }
  }
}

pub struct UIPresentation {
  pub primitives: Vec<Primitive>,
}

impl UIPresentation {
  pub fn new() -> Self {
    Self {
      primitives: Vec::new(),
    }
  }

  pub fn reset(&mut self) {
    self.primitives.clear()
  }
}
