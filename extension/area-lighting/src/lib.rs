use database::*;
// use reactive::*;
use rendiation_algebra::*;
use rendiation_scene_core::*;

pub fn register_area_lighting_data_model() {
  global_database()
    .declare_entity::<AreaLightEntity>()
    .declare_component::<AreaLightSize>()
    .declare_foreign_key::<AreaLightRefScene>()
    .declare_foreign_key::<AreaLightRefNode>();
}

declare_entity!(AreaLightEntity);
declare_foreign_key!(AreaLightRefScene, AreaLightEntity, SceneEntity);
declare_foreign_key!(AreaLightRefNode, AreaLightEntity, SceneNodeEntity);
declare_component!(AreaLightSize, AreaLightEntity, Vec2<f32>, Vec2::one()); // in meter
declare_component!(AreaLightIntensity, AreaLightEntity, Vec3<f32>, Vec3::one());
declare_component!(AreaLightIsRound, AreaLightEntity, bool, false); // in meter
declare_component!(AreaLightIsDoubleSide, AreaLightEntity, bool, false); // in meter

pub struct AreaLightDataView {
  pub size: Vec2<f32>,
  pub intensity: Vec3<f32>,
  pub is_round: bool,
  pub is_double_side: bool,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

impl AreaLightDataView {
  pub fn write(self, writer: &mut EntityWriter<AreaLightEntity>) -> EntityHandle<AreaLightEntity> {
    writer
      .component_value_writer::<AreaLightSize>(self.size)
      .component_value_writer::<AreaLightIntensity>(self.intensity)
      .component_value_writer::<AreaLightIsRound>(self.is_round)
      .component_value_writer::<AreaLightIsDoubleSide>(self.is_double_side)
      .component_value_writer::<AreaLightRefNode>(self.node.some_handle())
      .component_value_writer::<AreaLightRefScene>(self.scene.some_handle())
      .new_entity()
  }
}
