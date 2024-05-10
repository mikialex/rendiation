use std::{any::TypeId, marker::PhantomData};

use reactive::AllocIdx;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;

mod group;
pub use group::*;
mod model;
pub use model::*;
mod shape_helper;
pub use shape_helper::*;

pub trait View {
  fn update_view(&mut self, model: &mut StateReadStore);
  fn update_state(&mut self, cx: &mut View3dCtx);
}

pub struct StateReadStore {
  states: Vec<(TypeId, Option<*mut ()>)>,
}

impl StateReadStore {
  pub fn state<T, R>(&mut self, access: &StateTag<T>, f: impl FnOnce(&T, &mut Self) -> R) -> R {
    todo!()
  }

  pub fn register_state<T>(&mut self, v: &T, f: impl FnOnce(&mut Self)) {
    //
  }
}

pub struct StateWriteStore {
  states: Vec<(TypeId, Option<*mut ()>)>,
}

impl StateWriteStore {
  pub fn state<T>(&mut self, access: &StateTag<T>, f: impl FnOnce(&mut T, &mut Self)) {
    //
  }

  pub fn register_state<T>(&mut self, v: &mut T, f: impl FnOnce(&mut Self)) {
    //
  }
}

pub struct StateTag<T> {
  idx: usize,
  phantom: PhantomData<T>,
}

impl<T> Copy for StateTag<T> {}

impl<T> Clone for StateTag<T> {
  fn clone(&self) -> Self {
    Self {
      idx: self.idx.clone(),
      phantom: self.phantom.clone(),
    }
  }
}

impl<T> Default for StateTag<T> {
  fn default() -> Self {
    Self {
      idx: Default::default(),
      phantom: Default::default(),
    }
  }
}

pub struct View3dCtx<'a> {
  pub picker: &'a dyn Picker3d,
  pub mouse_world_ray: Ray3,
  pub is_mouse_left_down: bool,
  pub state: &'a mut StateWriteStore,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: AllocIdx<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}

pub struct StateCtxModify<T> {
  inner: T,
  v: usize,
}

impl<T> View for StateCtxModify<T> {
  fn update_view(&mut self, model: &mut StateReadStore) {
    todo!()
  }

  fn update_state(&mut self, cx: &mut View3dCtx) {
    // cx.state
    //   .state(access, |s, scx| scx.register_state(v, |scx| {}))
  }
}
