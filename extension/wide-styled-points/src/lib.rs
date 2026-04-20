#![feature(impl_trait_in_assoc_type)]

use database::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod gles_draw;
mod indirect_draw;
mod pick;
mod point_style;
pub use gles_draw::*;
pub use indirect_draw::*;
pub use pick::*;
use point_style::*;

pub fn register_wide_styled_points_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelWideStyledPointsRenderPayload>(sparse);

  let table = global_database()
    .declare_entity::<WideStyledPointsEntity>()
    .declare_component::<WideStyledPointsColor>()
    .declare_component::<WideStyledPointsMeshBuffer>();

  register_texture_with_sampling::<WidePointsColorAlphaTex>(table);
}

declare_foreign_key!(
  SceneModelWideStyledPointsRenderPayload,
  SceneModelEntity,
  WideStyledPointsEntity
);

declare_entity!(WideStyledPointsEntity);
declare_component!(
  WideStyledPointsMeshBuffer,
  WideStyledPointsEntity,
  ExternalRefPtr<Vec<u8>> // Vec<WideStyledPointVertex>
);
declare_component!(WideStyledPointsColor, WideStyledPointsEntity, Vec4<f32>);

declare_entity_associated!(WidePointsColorAlphaTex, WideStyledPointsEntity);
impl TextureWithSamplingForeignKeys for WidePointsColorAlphaTex {}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideStyledPointVertex {
  #[semantic(WidePointPosition)]
  pub position: Vec3<f32>,
  #[semantic(WidePointSize)]
  pub width: f32,
  #[semantic(WidePointStyleId)]
  pub style_id: u32,
}
