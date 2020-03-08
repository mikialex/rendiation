use crate::element::RenderCtx;
use super::{Element, Message};
use crate::element::ElementState;
use crate::{event::Event, renderer::GUIRenderer};
use rendiation_math::*;
use rendiation_render_entity::{Camera, OrthographicCamera};

pub struct QuadLayout {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl QuadLayout {
  pub fn is_point_in(&self, point: Vec2<f32>) -> bool {
    point.x >= self.x
      && point.y >= self.y
      && point.x <= self.x + self.width
      && point.y <= self.y + self.height
  }

  pub fn compute_matrix(&self, camera: &OrthographicCamera) -> Mat4<f32>{
    let scale_mat = Mat4::scale(self.width / 2., self.height / 2., 1.0);
    let position_mat  = Mat4::translate(-self.x, -self.y, 0.0);
    let model_mat = position_mat * scale_mat *  Mat4::translate(-1., -1., 0.0);
    let mvp = camera.get_vp_matrix() * model_mat;
    mvp
  }
}

pub struct Quad {
  pub quad: QuadLayout,
  pub color: Vec4<f32>,
  element_state: ElementState,
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
      element_state: ElementState::new(),
    }
  }
}

impl Element for Quad {
  fn render(&self, renderer: &mut RenderCtx) {
    let r = &mut renderer.renderer;
    r.draw_rect(&mut renderer.backend,&self.quad);
  }
  fn event(&self, event: &mut Message) {
    // decide if event need handled
  }
  fn get_element_state(&self) -> &ElementState {
    &self.element_state
  }
  fn is_point_in(&self, point: Vec2<f32>) -> bool {
    self.quad.is_point_in(point)
  }
}
