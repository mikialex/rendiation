use database::*;
use rendiation_scene_core::*;
use serde::*;

#[repr(C)]
#[derive(Serialize, Deserialize, Facet)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OccStyleZLayer {
  BotOSD = 0,
  #[default]
  Default = 1,
  Top = 2,
  TopMost = 3,
  TopOSD = 4,
}

declare_component!(SceneModelOccStyleLayer, SceneModelEntity, OccStyleZLayer);
declare_component!(SceneModelOccStylePriority, SceneModelEntity, u32);

pub fn register_occ_style_view_dependent_data_model() {
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelOccStyleLayer>();
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelOccStylePriority>();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OccStyleSceneContentKey {
  pub only_alpha_blend_objects: Option<bool>,
  pub layer: OccStyleZLayer,
}
