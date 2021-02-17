use super::{Element, Message};
use crate::element::ElementState;
use crate::element::RenderCtx;
use crate::{event::Event, renderer::GUIRenderer};
use rendiation_algebra::*;
use rendiation_render_entity::Camera;

pub struct Quad {
  pub quad: QuadLayout,
  pub color: Vec4<f32>,
}

impl Quad {
  pub fn new() -> Self {
    Self {
      quad: QuadLayout {
        width: 100.,
        height: 100.,
        x: 0.,
        y: 0.,
      },
      color: Vec4::new(1.0, 1.0, 1.0, 1.0),
    }
  }

  pub fn position(&mut self, x: f32, y: f32) -> &mut Self {
    self.quad.x = x;
    self.quad.y = y;
    self
  }

  pub fn size(&mut self, width: f32, height: f32) -> &mut Self {
    self.quad.width = width;
    self.quad.height = height;
    self
  }
}
