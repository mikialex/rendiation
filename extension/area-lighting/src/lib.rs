use database::*;
use rendiation_algebra::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_ltc::*;
use rendiation_lighting_transport::*;
use rendiation_scene_core::*;
use rendiation_shader_api::*;
use rendiation_shader_library::*;
use rendiation_texture_core::*;
use rendiation_texture_gpu_base::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod gles;
pub use gles::*;

pub fn register_area_lighting_data_model() {
  global_database()
    .declare_entity::<AreaLightEntity>()
    .declare_component::<AreaLightSize>()
    .declare_component::<AreaLightIntensity>()
    .declare_component::<AreaLightIsRound>()
    .declare_component::<AreaLightIsDoubleSide>()
    .declare_foreign_key::<AreaLightRefScene>()
    .declare_foreign_key::<AreaLightRefNode>();
}

declare_entity!(AreaLightEntity);
declare_foreign_key!(AreaLightRefScene, AreaLightEntity, SceneEntity);
declare_foreign_key!(AreaLightRefNode, AreaLightEntity, SceneNodeEntity);
declare_component!(
  /// The full width and height of the light shape in the local xy plane of the light node, in
  /// meters. The round light is the ellipse inscribed in this rectangle.
  AreaLightSize, AreaLightEntity, Vec2<f32>, Vec2::one());
declare_component!(
  /// The emitted radiance, in [nit](https://en.wikipedia.org/wiki/Candela_per_square_metre)
  /// (cd/m²), uniform over the light shape.
  AreaLightIntensity, AreaLightEntity, Vec3<f32>, Vec3::one());
declare_component!(
  /// Use the ellipse shape instead of the rectangle, see [AreaLightSize].
  AreaLightIsRound, AreaLightEntity, bool, false);
declare_component!(
  /// The single side light only emits toward the local -z of the light node (the same forward
  /// direction as the camera and the other lights), the double side light emits toward both sides.
  AreaLightIsDoubleSide, AreaLightEntity, bool, false);

pub struct AreaLightDataView {
  pub size: Vec2<f32>,
  pub intensity: Vec3<f32>,
  pub is_round: bool,
  pub is_double_side: bool,
  pub node: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
}

impl AreaLightDataView {
  pub fn write(self, writer: &mut TableWriter<AreaLightEntity>) -> EntityHandle<AreaLightEntity> {
    writer.new_entity(|w| {
      w.write::<AreaLightSize>(&self.size)
        .write::<AreaLightIntensity>(&self.intensity)
        .write::<AreaLightIsRound>(&self.is_round)
        .write::<AreaLightIsDoubleSide>(&self.is_double_side)
        .write::<AreaLightRefNode>(&self.node.some_handle())
        .write::<AreaLightRefScene>(&self.scene.some_handle())
    })
  }
}
