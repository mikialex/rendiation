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

/// state lives in self(internal state) or cx(external state passed in)
/// view lives in self(self present) or cx(outside view provider passed in)
pub trait StatefulView {
  /// foreach frame, view react to event and change state, event info is input from cx
  fn update_state(&mut self, cx: &mut StateCx);
  /// foreach frame, after update_state, state sync change to view and present to user
  fn update_view(&mut self, cx: &mut StateCx);
  /// should be called before self drop, do resource cleanup within the same cx in update cycle
  fn clean_up(&mut self, cx: &mut StateCx);
}

pub struct View3dProvider {}

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

pub trait ViewExt: StatefulView + Sized {
  fn with_view_update(self, f: impl FnMut(&mut Self, &mut StateCx)) -> impl StatefulView {
    ViewUpdate { inner: self, f }
  }
  fn with_state_update(self, f: impl FnMut(&mut StateCx)) -> impl StatefulView {
    StateUpdate {
      inner: self,
      f,
      post_update: false,
    }
  }
  fn with_state_post_update(self, f: impl FnMut(&mut StateCx)) -> impl StatefulView {
    StateUpdate {
      inner: self,
      f,
      post_update: true,
    }
  }
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl StatefulView {
    StateCtxInject { view: self, state }
  }
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl StatefulView {
    StateCtxPick {
      view: self,
      pick: len,
      phantom: PhantomData,
    }
  }
}

impl<T: StatefulView> ViewExt for T {}

pub struct ViewUpdate<T, F> {
  inner: T,
  f: F,
}

impl<T: StatefulView, F: FnMut(&mut T, &mut StateCx)> StatefulView for ViewUpdate<T, F> {
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

impl<T: StatefulView, F: FnMut(&mut StateCx)> StatefulView for StateUpdate<T, F> {
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

pub struct StateCxCreateOnce<T, F> {
  inner: Option<T>,
  f: F,
}

impl<T, F: Fn(&mut StateCx) -> T> StateCxCreateOnce<T, F> {
  pub fn new(f: F) -> Self {
    Self { inner: None, f }
  }
}

impl<T: StatefulView, F: Fn(&mut StateCx) -> T> StatefulView for StateCxCreateOnce<T, F> {
  fn update_state(&mut self, cx: &mut StateCx) {
    let inner = self.inner.get_or_insert_with(|| (self.f)(cx));
    inner.update_state(cx)
  }
  fn update_view(&mut self, cx: &mut StateCx) {
    let inner = self.inner.get_or_insert_with(|| (self.f)(cx));
    inner.update_view(cx)
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    if let Some(inner) = &mut self.inner {
      inner.clean_up(cx)
    }
  }
}
