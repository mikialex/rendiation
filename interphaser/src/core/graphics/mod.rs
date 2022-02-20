use crate::{TextCache, TextLayoutRef, UIPosition, UISize};

mod fonts;
pub use fonts::*;

mod path;
pub use path::*;

mod style;
pub use style::*;

mod shape;
pub use shape::*;

mod api;
pub use api::*;
use webgpu::GPUTexture2dView;

pub trait Presentable {
  fn render(&mut self, builder: &mut PresentationBuilder);
}

pub struct PresentationBuilder<'a> {
  pub fonts: &'a FontManager,
  pub texts: &'a mut TextCache,
  pub present: UIPresentation,
  parent_offset_chain: Vec<UIPosition>,
  current_origin_offset: UIPosition,
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

  pub fn current_origin_offset(&self) -> UIPosition {
    self.current_origin_offset
  }
}

#[derive(Clone)]
pub enum Style {
  SolidColor(Color),
  Texture(GPUTexture2dView),
}

#[derive(Clone)]
pub enum Primitive {
  Quad((Quad, Style)),
  Text(TextLayoutRef),
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
