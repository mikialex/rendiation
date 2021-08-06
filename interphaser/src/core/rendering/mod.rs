use crate::{LayoutSize, UIPosition};
use rendiation_algebra::*;
use std::rc::Rc;

mod fonts;
pub use fonts::*;

pub trait Presentable {
  fn render(&mut self, builder: &mut PresentationBuilder);
}

pub struct PresentationBuilder {
  pub present: UIPresentation,
  pub parent_offset_chain: Vec<UIPosition>,
  pub current_origin_offset: UIPosition,
}

impl PresentationBuilder {
  pub fn new() -> Self {
    Self {
      present: UIPresentation::new(),
      parent_offset_chain: Vec::new(),
      current_origin_offset: Default::default(),
    }
  }

  pub fn push_offset(&mut self, offset: UIPosition) {
    self.parent_offset_chain.push(offset);
    self.current_origin_offset.x += offset.x;
    self.current_origin_offset.y += offset.y;
  }

  pub fn pop_offset(&mut self) {
    if let Some(offset) = self.parent_offset_chain.pop() {
      self.current_origin_offset.x -= offset.x;
      self.current_origin_offset.y -= offset.y;
    }
  }
}

#[derive(Debug, Clone)]
pub enum Style {
  SolidColor(Vec4<f32>),
  Texture(Rc<wgpu::TextureView>),
}

#[derive(Debug, Clone)]
pub enum Primitive {
  Quad((Quad, Style)),
  Text(TextInfo),
}

#[derive(Debug, Clone, Default, Copy)]
pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl Quad {
  pub fn is_point_in(&self, p: UIPosition) -> bool {
    p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
  }
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
