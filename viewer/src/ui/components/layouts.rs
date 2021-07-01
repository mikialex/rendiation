use rendiation_algebra::*;

use crate::ui::{Component, Composer};

#[derive(PartialEq, Clone)]
pub struct Container {
  margin: Vec4<f32>,
  border: Vec4<f32>,
  padding: Vec4<f32>,
}

impl Default for Container {
  fn default() -> Self {
    Self {
      margin: Vec4::zero(),
      border: Vec4::zero(),
      padding: Vec4::zero(),
    }
  }
}

impl Component for Container {
  type State = ();
  fn build(&self, state: &Self::State, composer: &mut Composer<Self>) {
    // do nothing
  }
}

#[derive(Default, PartialEq, Clone)]
pub struct Row;

impl Component for Row {
  type State = ();
  fn build(&self, state: &Self::State, composer: &mut Composer<Self>) {
    // do nothing
  }
}
