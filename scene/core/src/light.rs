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

impl PointLightDataView {
  pub fn write(
    self,
    writer: &mut EntityWriter<PointLightEntity>,
  ) -> EntityHandle<PointLightEntity> {
    writer
      .component_value_writer::<PointLightIntensity>(self.intensity)
      .component_value_writer::<PointLightCutOffDistance>(self.cutoff_distance)
      .component_value_writer::<PointLightRefNode>(self.node.some_handle())
      .component_value_writer::<PointLightRefScene>(self.scene.some_handle())
      .new_entity()
  }
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
  pub half_penumbra_angle: f32,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

impl SpotLightDataView {
  pub fn write(self, writer: &mut EntityWriter<SpotLightEntity>) -> EntityHandle<SpotLightEntity> {
    writer
      .component_value_writer::<SplitLightIntensity>(self.intensity)
      .component_value_writer::<SpotLightCutOffDistance>(self.cutoff_distance)
      .component_value_writer::<SpotLightHalfConeAngle>(self.half_cone_angle)
      .component_value_writer::<SpotLightHalfPenumbraAngle>(self.half_penumbra_angle)
      .component_value_writer::<SpotLightRefNode>(self.node.some_handle())
      .component_value_writer::<SpotLightRefScene>(self.scene.some_handle())
      .new_entity()
  }
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

impl DirectionalLightDataView {
  pub fn write(
    self,
    writer: &mut EntityWriter<DirectionalLightEntity>,
  ) -> EntityHandle<DirectionalLightEntity> {
    writer
      .component_value_writer::<DirectionalLightIlluminance>(self.illuminance)
      .component_value_writer::<DirectionalRefNode>(self.node.some_handle())
      .component_value_writer::<DirectionalRefScene>(self.scene.some_handle())
      .new_entity()
  }
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
