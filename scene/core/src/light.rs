use crate::*;

pub const DEFAULT_CUTOFF_DISTANCE: f32 = 10.; // in meter

#[derive(Clone, Copy)]
pub enum SceneLightDataView {
  UnlitMaterial(EntityHandle<UnlitMaterialEntity>),
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
  pub fn write(self, writer: &mut TableWriter<PointLightEntity>) -> EntityHandle<PointLightEntity> {
    writer.new_entity(|w| {
      w.write::<PointLightIntensity>(&self.intensity)
        .write::<PointLightCutOffDistance>(&self.cutoff_distance)
        .write::<PointLightRefNode>(&self.node.some_handle())
        .write::<PointLightRefScene>(&self.scene.some_handle())
    })
  }
}

declare_entity!(
  /// A point light source.
  PointLightEntity);
declare_component!(
  /// Whether the point light is enabled.
  PointLightEnabled, PointLightEntity, bool, true);
declare_foreign_key!(
  /// Associates this light source with a [SceneEntity].
  ///
  /// The renderer lights the scene with this light source (along with any other associated
  /// light sources) when this association exists.
  PointLightRefScene, PointLightEntity, SceneEntity);
declare_foreign_key!(
  /// Determines the position of the light source. The position is the world space origin of the
  /// associated [SceneNodeEntity].
  PointLightRefNode, PointLightEntity, SceneNodeEntity);
declare_component!(
  /// The effective falloff distance of the light source, in meters.
  PointLightCutOffDistance,
  PointLightEntity,
  f32,
  DEFAULT_CUTOFF_DISTANCE
);
declare_component!(
  /// The intensity of the light source, in [cd](https://en.wikipedia.org/wiki/Candela).
  PointLightIntensity,
  PointLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
);

pub fn register_point_light_data_model() {
  global_database()
    .declare_entity::<PointLightEntity>()
    .declare_component::<PointLightEnabled>()
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
  pub fn write(self, writer: &mut TableWriter<SpotLightEntity>) -> EntityHandle<SpotLightEntity> {
    writer.new_entity(|w| {
      w.write::<SpotLightCutOffDistance>(&self.cutoff_distance)
        .write::<SpotLightHalfConeAngle>(&self.half_cone_angle)
        .write::<SpotLightHalfPenumbraAngle>(&self.half_penumbra_angle)
        .write::<SpotLightIntensity>(&self.intensity)
        .write::<SpotLightRefNode>(&self.node.some_handle())
        .write::<SpotLightRefScene>(&self.scene.some_handle())
    })
  }
}

declare_entity!(
  /// A spot light source.
  SpotLightEntity);
declare_component!(
  /// Whether the spot light is enabled.
  SpotLightEnabled, SpotLightEntity, bool, true);
declare_foreign_key!(
  /// Associates this light source with a [SceneEntity].
  SpotLightRefScene, SpotLightEntity, SceneEntity);
declare_foreign_key!(
  /// Determines the position and direction of the light source from the world space transform of
  /// the associated [SceneNodeEntity].
  SpotLightRefNode, SpotLightEntity, SceneNodeEntity);
declare_component!(
  /// The effective falloff distance of the light source, in meters.
  SpotLightCutOffDistance,
  SpotLightEntity,
  f32,
  DEFAULT_CUTOFF_DISTANCE
);
declare_component!(
  /// The half angle of the light cone, in radians.
  SpotLightHalfConeAngle, SpotLightEntity, f32, 0.5);
declare_component!(
  /// The half angle of the penumbra, in radians.
  SpotLightHalfPenumbraAngle, SpotLightEntity, f32, 0.5);
declare_component!(
  /// The intensity of the light source, in [cd](https://en.wikipedia.org/wiki/Candela).
  SpotLightIntensity,
  SpotLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
);

pub fn register_spot_light_data_model() {
  global_database()
    .declare_entity::<SpotLightEntity>()
    .declare_component::<SpotLightEnabled>()
    .declare_component::<SpotLightCutOffDistance>()
    .declare_component::<SpotLightHalfConeAngle>()
    .declare_component::<SpotLightHalfPenumbraAngle>()
    .declare_component::<SpotLightIntensity>()
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
    writer: &mut TableWriter<DirectionalLightEntity>,
  ) -> EntityHandle<DirectionalLightEntity> {
    writer.new_entity(|w| {
      w.write::<DirectionalLightIlluminance>(&self.illuminance)
        .write::<DirectionalRefNode>(&self.node.some_handle())
        .write::<DirectionalRefScene>(&self.scene.some_handle())
    })
  }
}

declare_entity!(
  /// A directional light source.
  DirectionalLightEntity);
declare_component!(
  /// Whether the directional light is enabled.
  DirectionalLightEnabled, DirectionalLightEntity, bool, true);

declare_component!(
  /// When enabled, the directional light follows the camera forward direction.
  ///
  /// todo, shadow not supported in this mode.
  DirectionalLightFollowCamera,
  DirectionalLightEntity,
  bool,
  false
);
declare_foreign_key!(
  /// Associates this light source with a [SceneEntity].
  DirectionalRefScene, DirectionalLightEntity, SceneEntity);
declare_foreign_key!(
  /// Determines the direction of the light source from the world space transform of the
  /// associated [SceneNodeEntity].
  DirectionalRefNode, DirectionalLightEntity, SceneNodeEntity);
declare_component!(
  /// The illuminance of the light source, in [lux](https://en.wikipedia.org/wiki/Lux).
  DirectionalLightIlluminance,
  DirectionalLightEntity,
  Vec3<f32>,
  Vec3::splat(100.)
);

pub fn register_directional_light_data_model() {
  global_database()
    .declare_entity::<DirectionalLightEntity>()
    .declare_component::<DirectionalLightEnabled>()
    .declare_component::<DirectionalLightFollowCamera>()
    .declare_component::<DirectionalLightIlluminance>()
    .declare_foreign_key::<DirectionalRefScene>()
    .declare_foreign_key::<DirectionalRefNode>();
}
