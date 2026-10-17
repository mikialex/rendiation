#![feature(iter_array_chunks)]
#![feature(allocator_api)]
//! Scene geometry query (picking).
//!
//! Every query is split into a hit producer and a hit consumer:
//!
//! - producers (TLAS candidate provider, scene model pickers, local model pickers) only know how
//!   to enumerate hits, they push hits into a visitor and never decide how hits are reduced.
//! - consumers (visitors) decide the reduction: nearest, all, any, k nearest, filtered, model level
//!   intersect or contains, sub primitives. They are written once and shared by every producer.
//!
//! Ray query visitors expose `t_max`, producers use it to prune BVH nodes, model boundings and
//! primitives. A filter visitor forwards `t_max` from its inner visitor, so a rejected hit never
//! shrinks the bound, which makes "nearest hit that passes the filter" correct without collecting
//! all hits first.
//!
//! Frustum query visitors receive per primitive pass or fail events. Model level verdicts are
//! reductions over these events (any pass for intersect, all pass for contains). Events carry a
//! primitive range so that a producer can report a whole model with a single event when its
//! bounding is fully inside the frustum.
//!
//! The layers are:
//!
//! - [SceneModelCandidateProvider]: the TLAS, enumerates candidate scene models of a scene.
//! - [SceneModelPicker]: scene model level, usually a decorator stack (view dependent transform,
//!   instancing) around [SceneModelPickerBaseImpl], which transforms the query into local space.
//! - [LocalModelPickerResolver]: selects and resolves the [ResolvedLocalModelPicker] of a model,
//!   which works in model local space for one kind of model data (attribute mesh, wide line,
//!   text, ...).
//! - query drivers in [query](crate::query) combine the above with a visitor.

use std::{
  alloc::Allocator,
  ops::{ControlFlow, Range},
};

use bytemuck::try_cast_slice;
use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
pub use rendiation_mesh_core::{IntersectTolerance, MeshBufferHitPoint, ToleranceType};
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

mod ray;
pub use ray::*;

mod frustum;
pub use frustum::*;

mod frustum_query;
pub use frustum_query::*;

mod picker;
pub use picker::*;

mod base;
pub use base::*;

mod local;
pub use local::*;

mod attribute;
pub use attribute::*;

mod candidate;
pub use candidate::*;

pub mod query;

declare_component!(SceneModelSelectable, SceneModelEntity, bool, true);
pub fn register_selectable_data_model() {
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelSelectable>();
}

/// The scene level ray query context. Filtering is done by wrapping the visitor with
/// [FilteredRayHits].
pub struct SceneRayQuery {
  pub world_ray: Ray3<f64>,
  pub camera_ctx: CameraQueryCtx,
  pub extra_screen_space_tolerance: f32,
}

pub struct SceneFrustumQuery {
  pub world_frustum: Frustum<f64>,
  pub world_helper: Option<FrustumIntersectionTestHelper<f64>>,
  pub camera_ctx: CameraQueryCtx,
  pub extra_screen_space_tolerance: f32,
}

pub struct CameraQueryCtx {
  pub camera_view_size_in_logic_pixel: Size,
  pub pixels_per_unit_calc: Box<dyn Fn(f32, f32) -> f32>,
  pub camera_world: Mat4<f64>,
  pub camera_vp: Mat4<f64>,
  pub camera_max_scale: f32,
}

impl CameraQueryCtx {
  pub fn compute_local_tolerance(
    &self,
    tolerance: IntersectTolerance,
    target_world_mat_max_scale: f64,
    target_object_center_in_world: Vec3<f64>,
  ) -> f32 {
    // take care of camera scale is important, user will likely using scale to control the orth camera view range.
    let mut local_tolerance =
      tolerance.value * self.camera_max_scale / target_world_mat_max_scale as f32;

    if let ToleranceType::ScreenSpace = tolerance.ty {
      let camera_to_target = target_object_center_in_world - self.camera_world.position();
      let projected_distance = camera_to_target.dot(self.camera_world.forward().reverse());
      let pixel_per_unit = (self.pixels_per_unit_calc)(
        projected_distance as f32,
        self.camera_view_size_in_logic_pixel.height_usize() as f32,
      );
      local_tolerance /= pixel_per_unit;
    }

    local_tolerance
  }
}

#[cfg(test)]
mod test;
