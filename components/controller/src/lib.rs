
#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]

pub mod orbit;
pub use orbit::*;
pub mod fps;
pub use fps::*;

use rendiation_algebra::Mat4;

pub trait Controller {
  fn update(&mut self, target: &mut dyn Transformed3DControllee) -> bool;
}

pub trait Transformed3DControllee {
  fn matrix(&self) -> &Mat4<f32>;
  fn matrix_mut(&mut self) -> &mut Mat4<f32>;
}

pub trait ControllerWinitEventSupport: Controller {
  type State: Default;
  fn event<T>(&mut self, state: &mut Self::State, event: &winit::event::Event<T>);
}

pub struct ControllerWinitAdapter<T: ControllerWinitEventSupport> {
  controller: T,
  state: T::State,
}

impl<T: ControllerWinitEventSupport> ControllerWinitAdapter<T> {
  pub fn new(controller: T) -> Self {
    Self {
      controller,
      state: T::State::default(),
    }
  }

  pub fn update(&mut self, target: &mut dyn Transformed3DControllee) -> bool {
    self.controller.update(target)
  }

  pub fn event<E>(&mut self, event: &winit::event::Event<E>) {
    self.controller.event(&mut self.state, event)
  }
}
