use facet::*;
use serde::*;

use crate::*;

pub trait BasicShadowMapConfigurable: EntityAssociateSemantic {}

pub struct BasicShadowMapResolutionOf<T>(T);
impl<T: BasicShadowMapConfigurable> EntityAssociateSemantic for BasicShadowMapResolutionOf<T> {
  type Entity = T::Entity;
}
impl<T: BasicShadowMapConfigurable> ComponentSemantic for BasicShadowMapResolutionOf<T> {
  type Data = Vec2<u32>;

  fn default_override() -> Self::Data {
    Vec2::new(256, 256)
  }
}

#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Facet)]
pub struct ShadowBiasConfig {
  pub bias: f32,
  pub normal_bias: f32,
}

use rendiation_lighting_shadow_map::ShadowBias;
impl From<ShadowBiasConfig> for ShadowBias {
  fn from(value: ShadowBiasConfig) -> Self {
    ShadowBias::new(value.bias, value.normal_bias)
  }
}

pub struct BasicShadowMapBiasOf<T>(T);
impl<T: BasicShadowMapConfigurable> EntityAssociateSemantic for BasicShadowMapBiasOf<T> {
  type Entity = T::Entity;
}
impl<T: BasicShadowMapConfigurable> ComponentSemantic for BasicShadowMapBiasOf<T> {
  type Data = ShadowBiasConfig;
}

pub struct BasicShadowMapEnabledOf<T>(T);
impl<T: BasicShadowMapConfigurable> EntityAssociateSemantic for BasicShadowMapEnabledOf<T> {
  type Entity = T::Entity;
}
impl<T: BasicShadowMapConfigurable> ComponentSemantic for BasicShadowMapEnabledOf<T> {
  type Data = bool;
  fn default_override() -> Self::Data {
    // default enable, why not?
    true
  }
}

pub fn register_basic_shadow_map_for_light<T: BasicShadowMapConfigurable>(
  ecg: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  ecg
    .declare_component::<BasicShadowMapResolutionOf<T>>()
    .declare_component::<BasicShadowMapBiasOf<T>>()
    .declare_component::<BasicShadowMapEnabledOf<T>>()
}

declare_component!(
  DirectionLightShadowBound,
  DirectionalLightEntity,
  Option<OrthographicProjection<f32>>
); // in meter

declare_entity_associated!(DirectionLightBasicShadowInfo, DirectionalLightEntity);
impl BasicShadowMapConfigurable for DirectionLightBasicShadowInfo {}

declare_entity_associated!(SpotLightBasicShadowInfo, SpotLightEntity);
impl BasicShadowMapConfigurable for SpotLightBasicShadowInfo {}

pub fn register_light_shadow_config() {
  let directional_light =
    global_entity_of::<DirectionalLightEntity>().declare_component::<DirectionLightShadowBound>();
  register_basic_shadow_map_for_light::<DirectionLightBasicShadowInfo>(directional_light);

  let spot_light = global_entity_of::<SpotLightEntity>();
  register_basic_shadow_map_for_light::<SpotLightBasicShadowInfo>(spot_light);
}
