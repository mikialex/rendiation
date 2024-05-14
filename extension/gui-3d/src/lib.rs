use std::any::Any;
use std::{any::TypeId, marker::PhantomData};

use database::*;
use fast_hash_collection::*;
use reactive::AllocIdx;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;

mod ty;
pub use ty::*;
mod state;
pub use state::*;
mod group;
pub use group::*;
mod model;
pub use model::*;
mod shape_helper;
pub use shape_helper::*;

pub struct View3dProvider {}

pub trait View3d {
  fn update_view(&mut self, cx: &mut StateCx);
  fn update_state(&mut self, cx: &mut StateCx);
  fn clean_up(&mut self, cx: &mut StateCx);
}

pub struct InteractionState3d {
  pub picker: Box<dyn Picker3d>,
  pub mouse_world_ray: Ray3,
  pub is_mouse_left_pressing: bool,
  pub is_mouse_left_releasing: bool,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: AllocIdx<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}

pub trait ViewExt: View3d {
  fn with_view_update(self, f: impl FnMut(&mut Self, &mut StateCx)) -> impl View3d;
  fn with_state_update(self, f: impl FnMut(&mut StateCx)) -> impl View3d;
  fn with_state_post_update(self, f: impl FnMut(&mut StateCx)) -> impl View3d;
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View3d;
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl View3d;
}

impl<T: View3d> ViewExt for T {
  fn with_view_update(self, f: impl FnMut(&mut T, &mut StateCx)) -> impl View3d {
    ViewUpdate { inner: self, f }
  }
  fn with_state_update(self, f: impl FnMut(&mut StateCx)) -> impl View3d {
    StateUpdate {
      inner: self,
      f,
      post_update: false,
    }
  }
  fn with_state_post_update(self, f: impl FnMut(&mut StateCx)) -> impl View3d {
    StateUpdate {
      inner: self,
      f,
      post_update: true,
    }
  }
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View3d {
    StateCtxInject { view: self, state }
  }
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl View3d {
    StateCtxPick {
      view: self,
      pick: len,
      phantom: PhantomData,
    }
  }
}

pub struct ViewUpdate<T, F> {
  inner: T,
  f: F,
}

impl<T: View3d, F: FnMut(&mut T, &mut StateCx)> View3d for ViewUpdate<T, F> {
  fn update_view(&mut self, cx: &mut StateCx) {
    (self.f)(&mut self.inner, cx);
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    self.inner.update_state(cx)
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.inner.clean_up(cx)
  }
}

pub struct StateUpdate<T, F> {
  inner: T,
  f: F,
  post_update: bool,
}

impl<T: View3d, F: FnMut(&mut StateCx)> View3d for StateUpdate<T, F> {
  fn update_view(&mut self, cx: &mut StateCx) {
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    if self.post_update {
      self.inner.update_state(cx);
      (self.f)(cx);
    } else {
      (self.f)(cx);
      self.inner.update_state(cx);
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.inner.clean_up(cx)
  }
}
