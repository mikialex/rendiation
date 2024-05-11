use std::any::Any;
use std::{any::TypeId, marker::PhantomData};

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

pub trait View {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx);
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx);
}

pub struct View3dViewUpdateCtx<'a> {
  pub state: &'a mut StateStore,
}

pub struct View3dStateUpdateCtx<'a> {
  pub picker: &'a dyn Picker3d,
  pub mouse_world_ray: Ray3,
  pub is_mouse_left_pressing: bool,
  pub is_mouse_left_releasing: bool,
  pub state: &'a mut StateStore,
  pub messages: MessageStore,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: AllocIdx<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}

pub trait ViewExt: View {
  fn with_view_update(self, f: impl FnMut(&mut Self, &mut View3dViewUpdateCtx)) -> impl View;
  fn with_state_update(self, f: impl FnMut(&mut View3dStateUpdateCtx)) -> impl View;
  fn with_state_post_update(self, f: impl FnMut(&mut View3dStateUpdateCtx)) -> impl View;
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View;
  fn with_state_pick<T1: 'static, T2: 'static>(self, len: impl Fn(&mut T1) -> &mut T2)
    -> impl View;
}

impl<T: View> ViewExt for T {
  fn with_view_update(self, f: impl FnMut(&mut T, &mut View3dViewUpdateCtx)) -> impl View {
    ViewUpdate { inner: self, f }
  }
  fn with_state_update(self, f: impl FnMut(&mut View3dStateUpdateCtx)) -> impl View {
    StateUpdate {
      inner: self,
      f,
      post_update: false,
    }
  }
  fn with_state_post_update(self, f: impl FnMut(&mut View3dStateUpdateCtx)) -> impl View {
    StateUpdate {
      inner: self,
      f,
      post_update: true,
    }
  }
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View {
    StateCtxInject { view: self, state }
  }
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl View {
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

impl<T: View, F: FnMut(&mut T, &mut View3dViewUpdateCtx)> View for ViewUpdate<T, F> {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    (self.f)(&mut self.inner, cx);
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    self.inner.update_state(cx)
  }
}

pub struct StateUpdate<T, F> {
  inner: T,
  f: F,
  post_update: bool,
}

impl<T: View, F: FnMut(&mut View3dStateUpdateCtx)> View for StateUpdate<T, F> {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    if self.post_update {
      self.inner.update_state(cx);
      (self.f)(cx);
    } else {
      (self.f)(cx);
      self.inner.update_state(cx);
    }
  }
}
