use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

impl PunctualShaderLight for SpotLightShaderInfo {
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let distance_factor =
      punctual_light_intensity_to_irradiance_factor(distance, light.cutoff_distance);

    let direction = direction.normalize();
    let angle_cos = direction.dot(light.direction);
    let angle_factor = angle_cos.smoothstep(light.half_cone_cos, light.half_penumbra_cos);

    ENode::<ShaderIncidentLight> {
      color: light.intensity * distance_factor * angle_factor,
      direction,
    }
  }
}

impl WebGPUSceneLight for SpotLight {
  fn collect(&self, sys: &mut ForwardLightingSystem, node: &SceneNode) {
    let lights = sys.get_or_create_list();

    let gpu = SpotLightShaderInfo {
      intensity: self.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      cutoff_distance: self.cutoff_distance,
      half_cone_cos: self.half_cone_angle.cos(),
      half_penumbra_cos: self.half_penumbra_angle.cos(),
      position: node.get_world_matrix().position(),
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}
