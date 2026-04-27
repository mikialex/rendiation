use std::hash::Hash;

use database::*;
use rendiation_scene_core::*;
use rendiation_shader_api::*;
use serde::*;
pub mod gles;
pub mod indirect;

declare_entity!(OccStyleMaterialEntity);
declare_component!(
  OccStyleMaterialTransparent,
  OccStyleMaterialEntity,
  bool,
  false
);
declare_component!(
  OccStyleMaterialDiffuse,
  OccStyleMaterialEntity,
  Vec4<f32>,
  Vec4::new(1.0, 1.0, 1.0, 1.0)
);
declare_component!(
  OccStyleMaterialSpecular,
  OccStyleMaterialEntity,
  Vec3<f32>,
  Vec3::new(1.0, 1.0, 1.0)
);

declare_component!(OccStyleMaterialShiness, OccStyleMaterialEntity, f32, 0.5);
declare_entity_associated!(OccStyleMaterialDiffuseTex, OccStyleMaterialEntity);
impl TextureWithSamplingForeignKeys for OccStyleMaterialDiffuseTex {}
declare_component!(
  OccStyleMaterialEmissive,
  OccStyleMaterialEntity,
  Vec3<f32>,
  Vec3::new(1.0, 1.0, 1.0)
);
declare_foreign_key!(
  OccStyleMaterialEffect,
  OccStyleMaterialEntity,
  OccStyleEffectControlEntity
);
declare_foreign_key!(
  StdModelOccStyleMaterialPayload,
  StandardModelEntity,
  OccStyleMaterialEntity
);

declare_entity!(OccStyleEffectControlEntity);

declare_component!(
  OccStyleEffectShadeType,
  OccStyleEffectControlEntity,
  OccStyleEffectType
);

#[repr(C)]
#[derive(Serialize, Deserialize, Facet)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OccStyleEffectType {
  Unlit,
  #[default]
  Lighted,
  Zebra,
}

pub fn register_occ_material_data_model(sparse: bool) {
  global_entity_of::<StandardModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<StdModelOccStyleMaterialPayload>(sparse);

  let table = global_database()
    .declare_entity::<OccStyleMaterialEntity>()
    .declare_component::<OccStyleMaterialTransparent>()
    .declare_component::<OccStyleMaterialDiffuse>()
    .declare_component::<OccStyleMaterialSpecular>()
    .declare_component::<OccStyleMaterialShiness>()
    .declare_component::<OccStyleMaterialEmissive>()
    .declare_foreign_key::<OccStyleMaterialEffect>();
  register_texture_with_sampling::<OccStyleMaterialDiffuseTex>(table);

  global_database()
    .declare_entity::<OccStyleEffectControlEntity>()
    .declare_component::<OccStyleEffectShadeType>();
}
