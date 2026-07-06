use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_scene_core::*;

mod bounding;
mod indirect_draw;
mod pick;

pub use bounding::use_instanced_model_local_bounding;
pub use indirect_draw::{
  use_transform_instanced_model_group_key, use_transform_instanced_model_indirect_renderer,
  TransformInstancedModelIndirectRenderer,
};
pub use pick::TransformInstancedMeshPicker;

declare_entity!(TransformInstancedModelEntity);
declare_component!(
/// Must contains at least one transform
TransformInstancedModelInstanceBuffer,
TransformInstancedModelEntity,
ExternalRefPtr<Vec<Mat4<f32>>>
);
declare_component!(
/// Each transform unit also affect(multiply) by this matrix.
/// This can impl effects such as per unit dynamic shrinking.
///
/// Instead of impl relative effect by user update all transform matrix,
/// this api can move the compute into gpu, trade the gpu time for update speed.
///
/// the unit transform is applied first
  TransformInstancedModelPerUnitTransform,
  TransformInstancedModelEntity,
  Option<Mat4<f32>>
);
declare_foreign_key!(
  /// The "source model"
  ///
  /// **It must has identity world matrix**, or the bounding compute is not correct,
  /// this is an implementation caveat.
  TransformInstancedModelRefSceneModel,
  TransformInstancedModelEntity,
  SceneModelEntity
);

declare_foreign_key!(
  SceneModelTransformInstancedModelPayload,
  SceneModelEntity,
  TransformInstancedModelEntity
);

pub fn register_transform_instanced_model_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelTransformInstancedModelPayload>(sparse);

  global_database()
    .declare_entity::<TransformInstancedModelEntity>()
    .declare_component::<TransformInstancedModelInstanceBuffer>()
    .declare_component::<TransformInstancedModelPerUnitTransform>()
    .declare_foreign_key::<TransformInstancedModelRefSceneModel>();
}
