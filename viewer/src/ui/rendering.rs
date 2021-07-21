use rendiation_algebra::*;

use crate::LayoutSize;

pub trait Presentable {
  fn render(&self, builder: &mut PresentationBuilder);
}

pub struct PresentationBuilder {
  pub present: UIPresentation,
}

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
  pub color: Vec4<f32>,
  pub font_size: f32,
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
  pub view_size: LayoutSize,
  pub primitives: Vec<Primitive>,
}

impl UIPresentation {
  pub fn new() -> Self {
    Self {
      primitives: Vec::new(),
      view_size: LayoutSize::new(1000., 1000.),
    }
  }

  pub fn reset(&mut self) {
    self.primitives.clear()
  }
}
