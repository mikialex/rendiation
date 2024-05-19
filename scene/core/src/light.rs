use crate::*;

#[derive(Clone, Copy)]
pub enum SceneLightDataView {
  FlatMaterial(EntityHandle<FlatMaterialEntity>),
  PbrSGMaterial(EntityHandle<PbrSGMaterialEntity>),
  PbrMRMaterial(EntityHandle<PbrMRMaterialEntity>),
}

pub struct PointLightDataView {
  pub intensity: Vec3<f32>,
  pub cutoff_distance: f32,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

declare_entity!(PointLightEntity);
declare_foreign_key!(PointLightRefScene, PointLightEntity, SceneEntity);
declare_foreign_key!(PointLightRefNode, PointLightEntity, SceneNodeEntity);
declare_component!(PointLightCutOffDistance, PointLightEntity, f32, 10.); // in meter
declare_component!(
  PointLightIntensity,
  PointLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
); // in cd

pub fn register_point_light_data_model() {
  global_database()
    .declare_entity::<PointLightEntity>()
    .declare_component::<PointLightCutOffDistance>()
    .declare_component::<PointLightIntensity>()
    .declare_foreign_key::<PointLightRefScene>()
    .declare_foreign_key::<PointLightRefNode>();
}

pub struct SpotLightDataView {
  pub intensity: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_angle: f32,
  pub penumbra_angle: f32,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

declare_entity!(SpotLightEntity);
declare_foreign_key!(SpotLightRefScene, SpotLightEntity, SceneEntity);
declare_foreign_key!(SpotLightRefNode, SpotLightEntity, SceneNodeEntity);
declare_component!(SpotLightCutOffDistance, SpotLightEntity, f32, 10.); // in meter
declare_component!(SpotLightHalfConeAngle, SpotLightEntity, f32, 0.5); // in rad
declare_component!(SpotLightHalfPenumbraAngle, SpotLightEntity, f32, 0.5); // in rad
declare_component!(
  SplitLightIntensity,
  SpotLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
); // in cd

pub fn register_spot_light_data_model() {
  global_database()
    .declare_entity::<SpotLightEntity>()
    .declare_component::<SpotLightCutOffDistance>()
    .declare_component::<SpotLightHalfConeAngle>()
    .declare_component::<SpotLightHalfPenumbraAngle>()
    .declare_component::<SplitLightIntensity>()
    .declare_foreign_key::<SpotLightRefScene>()
    .declare_foreign_key::<SpotLightRefNode>();
}

pub struct DirectionalLightDataView {
  pub illuminance: Vec3<f32>,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

declare_entity!(DirectionalLightEntity);
declare_foreign_key!(DirectionalRefScene, DirectionalLightEntity, SceneEntity);
declare_foreign_key!(DirectionalRefNode, DirectionalLightEntity, SceneNodeEntity);
declare_component!(
  DirectionalLightIlluminance,
  DirectionalLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
); // in lux

pub fn register_directional_light_data_model() {
  global_database()
    .declare_entity::<DirectionalLightEntity>()
    .declare_component::<DirectionalLightIlluminance>()
    .declare_foreign_key::<DirectionalRefScene>()
    .declare_foreign_key::<DirectionalRefNode>();
}
