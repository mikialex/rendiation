use crate::*;

declare_entity!(SceneLightEntity);
declare_foreign_key!(SceneLightBelongsToScene, SceneLightEntity, SceneEntity);
declare_foreign_key!(SceneLightNode, SceneLightEntity, SceneNodeEntity);

declare_foreign_key!(SceneLightPointLight, SceneLightEntity, PointLightEntity);
declare_foreign_key!(SceneLightSpotLight, SceneLightEntity, SpotLightEntity);
declare_foreign_key!(
  SceneLightDirectionalLight,
  SceneLightEntity,
  DirectionalLightEntity
);

pub fn register_light_data_model() {
  global_database()
    .declare_entity::<SceneLightEntity>()
    .declare_foreign_key::<SceneLightBelongsToScene>()
    .declare_foreign_key::<SceneLightNode>()
    .declare_foreign_key::<SceneLightPointLight>()
    .declare_foreign_key::<SceneLightSpotLight>()
    .declare_foreign_key::<SceneLightDirectionalLight>();
}

declare_entity!(PointLightEntity);
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
    .declare_component::<PointLightIntensity>();
}

declare_entity!(SpotLightEntity);
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
    .declare_component::<SplitLightIntensity>();
}

declare_entity!(DirectionalLightEntity);

declare_component!(
  DirectionalLightIlluminance,
  DirectionalLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
); // in lux

pub fn register_directional_light_data_model() {
  global_database()
    .declare_entity::<DirectionalLightEntity>()
    .declare_component::<DirectionalLightIlluminance>();
}
