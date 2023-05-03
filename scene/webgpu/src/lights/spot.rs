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
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<ENode<ShaderIncidentLight>, ShaderGraphBuildError> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let distance_factor =
      punctual_light_intensity_to_illuminance_factor(distance, light.cutoff_distance);

    let direction = direction.normalize();
    let angle_cos = direction.dot(light.direction);
    let angle_factor = angle_cos.smoothstep(light.half_cone_cos, light.half_penumbra_cos);

    let shadow_info = light.shadow.expand();
    let occlusion = consts(1.).mutable();

    let intensity_factor = distance_factor * angle_factor;

    if_by_ok(shadow_info.enabled.equals(consts(1)), || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();

      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).expand();

      let shadow_position = compute_shadow_position(builder, shadow_info)?;

      // we should have kept all light effective places inside the shadow volume
      if_by(intensity_factor.greater_than(consts(0.)), || {
        occlusion.set(sample_shadow(
          shadow_position,
          map,
          sampler,
          shadow_info.map_info,
        ))
      });
      Ok(())
    })?;

    let shadow_factor = consts(1.) - occlusion.get();

    Ok(ENode::<ShaderIncidentLight> {
      color: light.luminance_intensity * intensity_factor * shadow_factor,
      direction,
    })
  }
}

impl WebGPUSceneLight for SceneItemRef<SpotLight> {
  // allocate shadow maps
  fn pre_update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode) {
    let inner = self.read();
    request_basic_shadow_map(&inner, ctx.ctx.scene_resources, ctx.shadows, node);
  }

  fn update(&self, ctx: &mut LightUpdateCtx, node: &SceneNode) {
    let light = self.read();

    let shadow = check_update_basic_shadow_map(&light, ctx, node);

    let lights = ctx.forward.get_or_create_list();
    let world = ctx.node_derives.get_world_matrix(node);

    let gpu = SpotLightShaderInfo {
      luminance_intensity: light.luminance_intensity * light.color_factor,
      direction: world.forward().reverse().normalize(),
      cutoff_distance: light.cutoff_distance,
      half_cone_cos: light.half_cone_angle.cos(),
      half_penumbra_cos: light.half_penumbra_angle.cos(),
      position: world.position(),
      shadow,
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

impl ShadowCameraCreator for SpotLight {
  fn build_shadow_camera(&self, node: &SceneNode) -> SceneCamera {
    let proj = PerspectiveProjection {
      near: 0.1,
      far: 2000.,
      fov: Deg::from_rad(self.half_cone_angle * 2.),
      aspect: 1.,
    };
    SceneCamera::new(proj, node.clone())
  }
}
