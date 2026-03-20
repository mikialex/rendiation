#![feature(impl_trait_in_assoc_type)]

use database::*;
// use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod indirect_draw;
mod pick;
mod point_style;
pub use indirect_draw::*;
pub use pick::*;
use point_style::*;

pub fn register_wide_styled_points_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelWideStyledPointsRenderPayload>(sparse);

  global_database()
    .declare_entity::<WideStyledPointsEntity>()
    .declare_component::<WidesStyledPointsMeshBuffer>();
}

declare_foreign_key!(
  SceneModelWideStyledPointsRenderPayload,
  SceneModelEntity,
  WideStyledPointsEntity
);

declare_entity!(WideStyledPointsEntity);
declare_component!(
  WidesStyledPointsMeshBuffer,
  WideStyledPointsEntity,
  ExternalRefPtr<Vec<u8>> // Vec<WideLineVertex>
);
declare_component!(WidesStyledPointsColor, WideStyledPointsEntity, Vec3<f32>);

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct WideStyledPointVertex {
  pub position: Vec3<f32>,
  pub width: f32,
  pub style_id: u32,
}
