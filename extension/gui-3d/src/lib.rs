use std::{
  any::{Any, TypeId},
  marker::PhantomData,
};

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
  fn update_view(&mut self, model: &mut ViewStateStore);
  fn update_state(&mut self, cx: &mut View3dCtx);
}

pub struct ViewStateStore {
  states: Vec<(TypeId, Option<*mut ()>)>,
}

pub struct StateAccess<T> {
  idx: usize,
  phantom: PhantomData<T>,
}

impl<T> Copy for StateAccess<T> {}

impl<T> Clone for StateAccess<T> {
  fn clone(&self) -> Self {
    Self {
      idx: self.idx.clone(),
      phantom: self.phantom.clone(),
    }
  }
}

impl<T> Default for StateAccess<T> {
  fn default() -> Self {
    Self {
      idx: Default::default(),
      phantom: Default::default(),
    }
  }
}

impl ViewStateStore {
  pub fn state<T>(&mut self, access: &StateAccess<T>, f: impl FnOnce(&T, &mut Self)) {
    //
  }

  pub fn register_state<T>(&mut self, v: &T, f: impl FnOnce(&mut Self)) {
    //
  }
}

pub struct View3dCtx<'a> {
  pub picker: &'a dyn Picker3d,
  pub mouse_world_ray: Ray3,
  pub is_mouse_left_down: bool,
  pub state: &'a ViewStateStore,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: AllocIdx<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}
