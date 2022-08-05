#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]

pub mod orbit;
pub use orbit::*;
pub mod fps;
pub use fps::*;

use rendiation_algebra::{Mat4, Vec2};

pub trait Controller {
  /// Sync the controller state to target state
  ///
  /// After sync, if update triggered, should not change the target's state
  ///
  /// This is useful when controller init controllee or controllee switch between
  /// different controllers
  fn sync(&mut self, target: &dyn Transformed3DControllee);

  /// update target states and return if state has actually changed
  fn update(&mut self, target: &mut dyn Transformed3DControllee) -> bool;
}

pub trait Transformed3DControllee {
  fn get_matrix(&self) -> Mat4<f32>;
  fn set_matrix(&mut self, m: Mat4<f32>);
}

pub struct InputBound {
  pub origin: Vec2<f32>,
  pub size: Vec2<f32>,
}

impl InputBound {
  pub fn is_point_in(&self, point: Vec2<f32>) -> bool {
    point.x >= self.origin.x
      && point.y >= self.origin.y
      && point.x <= self.origin.x + self.size.x
      && point.y <= self.origin.y + self.size.y
  }
}

pub trait ControllerWinitEventSupport: Controller {
  type State: Default;

  fn event<T>(
    &mut self,
    state: &mut Self::State,
    event: &winit::event::Event<T>,
    bound: InputBound,
  );
}

pub struct ControllerWinitAdapter<T: ControllerWinitEventSupport> {
  controller: T,
  state: T::State,
  last_sync: Option<Mat4<f32>>,
}

impl<T: ControllerWinitEventSupport> ControllerWinitAdapter<T> {
  pub fn new(controller: T) -> Self {
    Self {
      controller,
      state: T::State::default(),
      last_sync: Default::default(),
    }
  }

  pub fn update(&mut self, target: &mut dyn Transformed3DControllee) -> bool {
    // check if the synced mat is not the last time we modified
    if let Some(last_sync) = self.last_sync {
      if last_sync != target.get_matrix() {
        self.controller.sync(target)
      }
    } else {
      self.controller.sync(target)
    }

    let changed = self.controller.update(target);

    self.last_sync = (target.get_matrix()).into();

    changed
  }

  pub fn event<E>(&mut self, event: &winit::event::Event<E>, bound: InputBound) {
    self.controller.event(&mut self.state, event, bound)
  }
}
