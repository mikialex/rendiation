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
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx);
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx);
}

pub struct StateReadStore {
  states: Vec<(TypeId, Option<*mut ()>)>,
}

impl StateReadStore {
  pub fn state<T, R>(&mut self, tag: &StateTag<T>, f: impl FnOnce(&T) -> R) -> R {
    todo!()
  }

  pub unsafe fn register_state<T>(&mut self, tag: &mut StateTag<T>, v: &mut T) {
    //
  }

  pub unsafe fn unregister_state<T>(&mut self, tag: &mut StateTag<T>) {
    //
  }
}

pub struct StateWriteStore {
  states: Vec<(TypeId, Option<*mut ()>)>,
}

impl StateWriteStore {
  pub fn state<T, R>(&mut self, access: &StateTag<T>, f: impl FnOnce(&mut T) -> R) -> R {
    todo!()
  }

  pub unsafe fn register_state<T>(&mut self, tag: &StateTag<T>, v: &T) {
    //
  }

  pub unsafe fn unregister_state<T>(&mut self, tag: &StateTag<T>) {
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

pub struct View3dViewUpdateCtx<'a> {
  pub state: &'a mut StateReadStore,
}

pub struct View3dStateUpdateCtx<'a> {
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

pub struct StateCtxInject<T, V> {
  view: V,
  state: T,
  tag: StateTag<T>,
}

impl<T, V: View> View for StateCtxInject<T, V> {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    unsafe {
      cx.state.register_state(&mut self.tag, &mut self.state);
      self.view.update_view(cx);
      cx.state.unregister_state(&mut self.tag)
    }
  }

  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    unsafe {
      cx.state.register_state(&mut self.tag, &self.state);
      self.view.update_state(cx);
      cx.state.unregister_state(&mut self.tag)
    }
  }
}
