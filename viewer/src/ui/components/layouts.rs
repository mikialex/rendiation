use rendiation_algebra::*;

use crate::ui::*;

#[derive(PartialEq, Clone)]
pub struct Container {
  margin: Vec4<f32>,
  border: Vec4<f32>,
  padding: Vec4<f32>,
  width: Option<f32>,
  height: Option<f32>,
}

impl Default for Container {
  fn default() -> Self {
    Self {
      margin: Vec4::zero(),
      border: Vec4::zero(),
      padding: Vec4::zero(),
      width: None,
      height: None,
    }
  }
}

impl Component for Container {
  type State = ();
}

#[derive(Default, PartialEq, Clone)]
pub struct Row;

impl Component for Row {
  type State = ();
  fn build(model: &mut Model<Self>, c: &mut Composer<Self>) {
    // do nothing
  }
}
