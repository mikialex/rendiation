use std::{any::TypeId, marker::PhantomData};

use fast_hash_collection::*;
use reactive::AllocIdx;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;

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
  pub is_mouse_left_down: bool,
  pub state: &'a mut StateStore,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: AllocIdx<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}
