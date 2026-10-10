use gltf_json::extensions::scene::khr_lights_punctual::{Light, Spot, Type};
use gltf_json::validation::Checked;
use rendiation_algebra::*;

use crate::*;

/// the enabled lights of the scene and the nodes they attach to, gltf has no disabled light
pub fn collect_scene_lights(
  scene: EntityHandle<SceneEntity>,
) -> Vec<(EntityHandle<SceneNodeEntity>, Light)> {
  let scene = Some(scene.into_raw());
  let mut lights = Vec::new();
  let mut push = |node: Option<RawEntityHandle>, light| {
    if let Some(node) = node {
      lights.push((unsafe { EntityHandle::from_raw(node) }, light));
    }
  };

  let enabled = get_db_view::<DirectionalLightEnabled>();
  let node = get_db_view::<DirectionalRefNode>();
  let illuminance = get_db_view::<DirectionalLightIlluminance>();
  for (id, light_scene) in get_db_view::<DirectionalRefScene>().iter_key_value() {
    if light_scene == scene && enabled.access(&id).unwrap() {
      let light = build_light(
        Type::Directional,
        illuminance.access(&id).unwrap(),
        None,
        None,
      );
      push(node.access(&id).unwrap(), light);
    }
  }

  let enabled = get_db_view::<PointLightEnabled>();
  let node = get_db_view::<PointLightRefNode>();
  let intensity = get_db_view::<PointLightIntensity>();
  let cutoff = get_db_view::<PointLightCutOffDistance>();
  for (id, light_scene) in get_db_view::<PointLightRefScene>().iter_key_value() {
    if light_scene == scene && enabled.access(&id).unwrap() {
      let light = build_light(
        Type::Point,
        intensity.access(&id).unwrap(),
        cutoff.access(&id),
        None,
      );
      push(node.access(&id).unwrap(), light);
    }
  }

  let enabled = get_db_view::<SpotLightEnabled>();
  let node = get_db_view::<SpotLightRefNode>();
  let intensity = get_db_view::<SpotLightIntensity>();
  let cutoff = get_db_view::<SpotLightCutOffDistance>();
  let half_cone_angle = get_db_view::<SpotLightHalfConeAngle>();
  let half_inner_cone_angle = get_db_view::<SpotLightHalfInnerConeAngle>();
  for (id, light_scene) in get_db_view::<SpotLightRefScene>().iter_key_value() {
    if light_scene == scene && enabled.access(&id).unwrap() {
      // gltf requires the outer angle not larger than half pi, and the inner angle less than it
      let outer = half_cone_angle
        .access(&id)
        .unwrap()
        .min(std::f32::consts::FRAC_PI_2);
      let inner = half_inner_cone_angle
        .access(&id)
        .unwrap()
        .clamp(0., (outer - 0.0001).max(0.));
      let spot = Spot {
        inner_cone_angle: inner,
        outer_cone_angle: outer,
      };
      let light = build_light(
        Type::Spot,
        intensity.access(&id).unwrap(),
        cutoff.access(&id),
        Some(spot),
      );
      push(node.access(&id).unwrap(), light);
    }
  }

  lights
}

/// the scene light intensity is the gltf color multiplied by the gltf intensity
fn build_light(type_: Type, intensity: Vec3<f32>, range: Option<f32>, spot: Option<Spot>) -> Light {
  let max = intensity.max_channel();
  let color = if max > 0. {
    intensity / max
  } else {
    Vec3::one()
  };
  Light {
    color: color.into(),
    extensions: Default::default(),
    extras: Default::default(),
    intensity: max,
    name: Default::default(),
    range,
    spot,
    type_: Checked::Valid(type_),
  }
}
