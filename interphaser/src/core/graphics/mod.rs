use crate::{TextCache, TextLayoutRef, UIPosition, UISize};
use std::rc::Rc;

mod fonts;
pub use fonts::*;

mod path;
pub use path::*;

mod style;
pub use style::*;

mod api;
pub use api::*;

pub trait Presentable {
  fn render(&mut self, builder: &mut PresentationBuilder);
}

pub struct PresentationBuilder<'a> {
  pub fonts: &'a FontManager,
  pub texts: &'a mut TextCache,
  pub present: UIPresentation,
  pub parent_offset_chain: Vec<UIPosition>,
  pub current_origin_offset: UIPosition,
}

impl<'a> PresentationBuilder<'a> {
  pub fn new(fonts: &'a FontManager, texts: &'a mut TextCache) -> Self {
    Self {
      fonts,
      texts,
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
  SolidColor(Color),
  Texture(Rc<wgpu::TextureView>),
}

#[derive(Clone)]
pub enum Primitive {
  Quad((Quad, Style)),
  Text(TextLayoutRef),
}

#[derive(Debug, Clone, Default, Copy)]
pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RoundCorneredQuad {
  pub quad: Quad,
  pub radius: RadiusGroup,
}

pub enum QuadRadius {
  No,
  All(f32),
  Four(RadiusGroup),
}

impl Default for QuadRadius {
  fn default() -> Self {
    Self::No
  }
}

#[derive(Default)]
pub struct QuadBoundaryWidth {
  pub top: f32,
  pub bottom: f32,
  pub left: f32,
  pub right: f32,
}

#[derive(Default)]
pub struct QuadBorder {
  pub radius: QuadRadius,
  pub width: QuadBoundaryWidth,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct RadiusGroup {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_left: f32,
  pub bottom_right: f32,
}

impl Quad {
  pub fn is_point_in(&self, p: UIPosition) -> bool {
    p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
  }
}

pub struct UIPresentation {
  pub view_size: UISize,
  pub primitives: Vec<Primitive>,
}

impl UIPresentation {
  pub fn new() -> Self {
    Self {
      primitives: Vec::new(),
      view_size: UISize::new(1000., 1000.),
    }
  }

  pub fn reset(&mut self) {
    self.primitives.clear()
  }
}

impl Default for UIPresentation {
  fn default() -> Self {
    Self::new()
  }
}
