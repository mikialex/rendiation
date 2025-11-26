#![feature(impl_trait_in_assoc_type)]

use database::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod draw;
use draw::*;

mod pick;
pub use pick::*;

mod gles_draw;
pub use gles_draw::*;

mod indirect_draw;
pub use indirect_draw::*;

pub fn register_wide_line_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelWideLineRenderPayload>(sparse);

  global_database()
    .declare_entity::<WideLineModelEntity>()
    .declare_component::<WideLineWidth>()
    .declare_component::<WideLineMeshBuffer>();
}

declare_foreign_key!(
  SceneModelWideLineRenderPayload,
  SceneModelEntity,
  WideLineModelEntity
);

declare_entity!(WideLineModelEntity);
declare_component!(WideLineWidth, WideLineModelEntity, f32, 1.0);
declare_component!(
  WideLineMeshBuffer,
  WideLineModelEntity,
  ExternalRefPtr<Vec<u8>> // Vec<WideLineVertex>
);

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideLineVertex {
  #[semantic(WideLineStart)]
  pub start: Vec3<f32>,
  #[semantic(WideLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}
