use database::*;
use rendiation_algebra::*;
use rendiation_scene_core::SceneModelEntity;

mod slug_shader;

pub fn register_text3d_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelText3dPayload>(sparse);

  global_database()
    .declare_entity::<Text3dEntity>()
    .declare_component::<Text3dContent>();
}

declare_foreign_key!(SceneModelText3dPayload, SceneModelEntity, Text3dEntity);

declare_entity!(Text3dEntity);
declare_component!(Text3dContent, Text3dEntity, ExternalRefPtr<String>);
declare_component!(Text3dFont, Text3dEntity, Option<u32>);
declare_component!(Text3dWeight, Text3dEntity, Option<u32>);
declare_component!(Text3dColor, Text3dEntity, Vec3<f32>, Vec3::zero());

pub struct FontManager {}
