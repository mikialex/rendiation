use reactive::AllocIdx;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;
pub use widget::*;

mod ty;
pub use ty::*;
mod group;
pub use group::*;
mod model;
pub use model::*;
mod shape_helper;
pub use shape_helper::*;

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
