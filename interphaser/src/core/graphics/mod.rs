use rendiation_texture::Size;

use crate::*;

pub trait UIPresenter {
  fn resize(&mut self, size: Size);
  fn render(&mut self, content: &UIPresentation, fonts: &FontManager, texts: &mut TextCache);
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

  pub fn push_translate(&mut self, offset: UIPosition) {
    self.parent_offset_chain.push(offset);
    self.current_origin_offset.x += offset.x;
    self.current_origin_offset.y += offset.y;
  }

  pub fn pop_translate(&mut self) {
    if let Some(offset) = self.parent_offset_chain.pop() {
      self.current_origin_offset.x -= offset.x;
      self.current_origin_offset.y -= offset.y;
    }
  }

  pub fn current_absolution_origin(&self) -> UIPosition {
    self.current_origin_offset
  }
}

#[derive(Clone)]
pub enum Style {
  SolidColor(DisplayColor),
  Texture(GPU2DTextureView),
}

#[derive(Clone)]
pub enum Primitive {
  Quad((RectangleShape, Style)),
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
