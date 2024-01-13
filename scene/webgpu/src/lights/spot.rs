use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
  pub shadow: LightShadowAddressInfo,
}

impl PunctualShaderLight for SpotLightShaderInfo {
  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let distance_factor =
      punctual_light_intensity_to_illuminance_factor_fn(distance, light.cutoff_distance);

    let direction = direction.normalize();
    let angle_cos = direction.dot(light.direction);
    let angle_factor = angle_cos.smoothstep(light.half_cone_cos, light.half_penumbra_cos);

    let shadow_info = light.shadow.expand();
    let occlusion = val(1.).make_local_var();

    let intensity_factor = distance_factor * angle_factor;

    if_by_ok(shadow_info.enabled.equals(1), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).load().expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      // we should have kept all light effective places inside the shadow volume
      if_by(intensity_factor.greater_than(0.), || {
        occlusion.store(sample_shadow(
          shadow_position,
          map,
          sampler,
          shadow_info.map_info,
        ))
      });
      Ok(())
    })
    .unwrap();

    let shadow_factor = val(1.) - occlusion.load();

    ENode::<ShaderIncidentLight> {
      color: light.luminance_intensity * intensity_factor * shadow_factor,
      direction,
    }
  }
}

fn build_spot_lights_shadow_projections(
) -> impl ReactiveCollection<AllocIdx<SpotLight>, (CameraProjectionEnum, Size)> {
  storage_of::<SpotLight>()
    .listen_to_reactive_collection(|| Some(()))
    .collective_execute_map_by(|| {
      let compute = storage_of::<SpotLight>().create_key_mapper(|light| {
        let proj = PerspectiveProjection {
          near: 0.1,
          far: 2000.,
          fov: Deg::from_rad(light.read().half_cone_angle * 2.),
          aspect: 1.,
        };
        let proj = CameraProjectionEnum::Perspective(proj);
        let size = Size::from_u32_pair_min_one((512, 512));
        (proj, size)
      });
      move |k, _| compute(*k)
    })
}

fn build_spot_lights_shadow_info(
  shadow_sys: &SingleProjectShadowMapSystem,
  shadows: impl ReactiveCollection<AllocIdx<SpotLight>, (CameraProjectionEnum, Size)>,
) -> impl ReactiveCollection<AllocIdx<SpotLight>, LightShadowAddressInfo> {
  // todo
}

fn spot_lights_directions_position(
  node_mats: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  light_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<SpotLight>>,
) -> impl ReactiveCollection<AllocIdx<SpotLight>, (Vec3<f32>, Vec3<f32>)> {
  // this is costly because the light count is always very small in a scene
  node_mats
    .one_to_many_fanout(light_node_relation)
    .collective_map(|mat| (mat.forward().reverse().normalize(), mat.position()))
}

struct SpotLightShaderInfoPart {
  pub luminance_intensity: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}
fn dir_spots_ill() -> impl ReactiveCollection<AllocIdx<SpotLight>, SpotLightShaderInfoPart> {
  storage_of::<SpotLight>()
    .listen_to_reactive_collection(|| Some(()))
    .collective_execute_map_by(|| {
      let compute = storage_of::<SpotLight>().create_key_mapper(|light| SpotLightShaderInfoPart {
        luminance_intensity: light.luminance_intensity * light.color_factor,
        cutoff_distance: light.cutoff_distance,
        half_cone_cos: light.half_cone_angle.cos(),
        half_penumbra_cos: light.half_penumbra_angle.cos(),
      });
      move |k, _| compute(*k)
    })
}

fn spot_lights_uniform(
  info: impl ReactiveCollection<AllocIdx<SpotLight>, SpotLightShaderInfoPart>,
  dp: impl ReactiveCollection<AllocIdx<SpotLight>, (Vec3<f32>, Vec3<f32>)>,
  shadow: impl ReactiveCollection<AllocIdx<SpotLight>, LightShadowAddressInfo>,
) -> impl ReactiveCollection<AllocIdx<SpotLight>, SpotLightShaderInfo> {
  info
    .collective_zip(dp)
    .collective_zip(shadow)
    .collective_map(|((info, dp), shadow)| SpotLightShaderInfo {
      luminance_intensity: info.luminance_intensity,
      cutoff_distance: info.cutoff_distance,
      half_penumbra_cos: info.half_penumbra_cos,
      half_cone_cos: info.half_cone_cos,
      direction: dp.0,
      position: dp.1,
      shadow,
      ..Default::default()
    })
}
