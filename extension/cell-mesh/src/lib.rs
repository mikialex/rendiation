use database::*;
use rendiation_algebra::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;

mod indirect_draw;
pub use indirect_draw::{use_cell_mesh_group_key, use_cell_mesh_renderer};

mod pick;
pub use pick::*;

declare_entity!(CellMeshEntity);
declare_component!(CellMeshDisplayMode2D, CellMeshEntity, bool, false);

/// This is a unit of cell. A cell is a quad or triangle face.
///
/// For convenience, the triangle is also implemented as a quad(with degenerated one triangle).
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
#[derive(Serialize, Deserialize, PartialEq, Facet)]
pub struct CellMeshUnitData {
  // the position of the four vertices
  pub p1: Vec3<f32>,
  pub p2: Vec3<f32>,
  pub p3: Vec3<f32>,
  pub p4: Vec3<f32>,
  // the shrink center of this unit
  pub center: Vec3<f32>,

  pub front_face_color: Vec3<f32>,
  pub back_face_color: Vec3<f32>,
}

declare_component!(
  CellMeshUnitsBuffer,
  CellMeshEntity,
  ExternalRefPtr<Vec<CellMeshUnitData>>
);
declare_component!(CellMeshShrinkRatio, CellMeshEntity, f32);
declare_foreign_key!(
  StandardModelCellMeshPayload,
  StandardModelEntity,
  CellMeshEntity
);

pub fn register_cell_mesh_data_model(sparse: bool) {
  global_entity_of::<StandardModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<StandardModelCellMeshPayload>(sparse);

  global_database()
    .declare_entity::<CellMeshEntity>()
    .declare_component::<CellMeshDisplayMode2D>()
    .declare_component::<CellMeshUnitsBuffer>()
    .declare_component::<CellMeshShrinkRatio>();
}
