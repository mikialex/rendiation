use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub shadow: LightShadowAddressInfo,
}

impl PunctualShaderLight for DirectionalLightShaderInfo {
  fn compute_incident_light(
    builder: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let shadow_info = light.shadow.expand();
    let occlusion = val(1.).make_local_var();

    if_by_ok(shadow_info.enabled.equals(1), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).load().expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      if_by(cull_directional_shadow(shadow_position), || {
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

    ENode::<ShaderIncidentLight> {
      color: light.illuminance * (val(1.) - occlusion.load()),
      direction: light.direction,
    }
  }
}

/// custom extra culling for directional light
fn cull_directional_shadow(shadow_position: Node<Vec3<f32>>) -> Node<bool> {
  let left = shadow_position.x().greater_equal_than(val(0.));
  let right = shadow_position.x().less_equal_than(val(1.));
  let top = shadow_position.y().greater_equal_than(val(0.));
  let bottom = shadow_position.y().less_equal_than(val(1.));
  let far = shadow_position.z().less_equal_than(val(1.));

  left.and(right).and(top).and(bottom).and(far)
}

#[derive(Copy, Clone)]
pub struct DirectionalShadowMapExtraInfo {
  pub range: OrthographicProjection<f32>,
  // pub enable_shadow: bool,
}

impl Default for DirectionalShadowMapExtraInfo {
  fn default() -> Self {
    Self {
      range: OrthographicProjection {
        left: -20.,
        right: 20.,
        top: 20.,
        bottom: -20.,
        near: 0.1,
        far: 2000.,
      },
    }
  }
}

fn build_dir_lights_shadow_projections(
) -> impl ReactiveCollection<AllocIdx<DirectionalLight>, (Mat4<f32>, Size)> {
  storage_of::<DirectionalLight>()
    .listen_to_reactive_collection(|| Some(()))
    .collective_execute_map_by(|| {
      let compute = storage_of::<DirectionalLight>().create_key_mapper(|light| {
        let shadow_info = light
          .ext
          .get::<DirectionalShadowMapExtraInfo>()
          .cloned()
          .unwrap_or_default();
        let size = Size::from_u32_pair_min_one((512, 512)); // todo

        (shadow_info.range.compute_projection_mat::<WebGPU>(), size)
      });
      move |k, _| compute(*k)
    })
}

fn build_dir_lights_shadow_info(
  shadow_sys: &SingleProjectShadowMapSystem,
  shadows: impl ReactiveCollection<AllocIdx<DirectionalLight>, (Mat4<f32>, Size)>,
) -> impl ReactiveCollection<AllocIdx<DirectionalLight>, LightShadowAddressInfo> {
  // todo
}

fn dir_lights_directions(
  node_mats: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  light_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<DirectionalLight>>,
) -> impl ReactiveCollection<AllocIdx<DirectionalLight>, Vec3<f32>> {
  // this is costly because the light count is always very small in a scene
  node_mats
    .one_to_many_fanout(light_node_relation)
    .collective_map(|mat| mat.forward().reverse().normalize())
}

fn dir_lights_intensity() -> impl ReactiveCollection<AllocIdx<DirectionalLight>, Vec3<f32>> {
  storage_of::<DirectionalLight>()
    .listen_to_reactive_collection(|| Some(()))
    .collective_execute_map_by(|| {
      let compute = storage_of::<DirectionalLight>()
        .create_key_mapper(|light| light.illuminance * light.color_factor);
      move |k, _| compute(*k)
    })
}

fn dir_lights_uniform(
  intensity: impl ReactiveCollection<AllocIdx<DirectionalLight>, Vec3<f32>>,
  directions: impl ReactiveCollection<AllocIdx<DirectionalLight>, Vec3<f32>>,
  shadow: impl ReactiveCollection<AllocIdx<DirectionalLight>, LightShadowAddressInfo>,
) -> impl ReactiveCollection<AllocIdx<DirectionalLight>, DirectionalLightShaderInfo> {
  intensity
    .collective_zip(directions)
    .collective_zip(shadow)
    .collective_map(
      |((intensity, direction), shadow)| DirectionalLightShaderInfo {
        illuminance: intensity,
        direction,
        shadow,
        ..Default::default()
      },
    );
}
