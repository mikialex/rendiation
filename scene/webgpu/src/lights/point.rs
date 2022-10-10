use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

impl PunctualShaderLight for PointLightShaderInfo {
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    _: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let factor = punctual_light_intensity_to_irradiance_factor(distance, light.cutoff_distance);

    ENode::<ShaderIncidentLight> {
      color: light.intensity * factor,
      direction: direction.normalize(),
    }
  }
}

impl WebGPUSceneLight for SceneLight<PointLight> {
  fn update(&self, ctx: &mut LightUpdateCtx) {
    let inner = self.read();
    let light = &inner.light;
    let node = &inner.node;

    let lights = ctx.forward.get_or_create_list();

    let gpu = PointLightShaderInfo {
      intensity: light.intensity,
      position: node.get_world_matrix().position(),
      cutoff_distance: light.cutoff_distance,
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

wgsl_fn!(
  // based upon Frostbite 3 Moving to Physically-based Rendering
  // page 32, equation 26: E[window1]
  // https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
  // this is intended to be used on spot and point lights who are represented as luminous intensity
  // but who must be converted to luminous irradiance for surface lighting calculation
  fn punctual_light_intensity_to_irradiance_factor(
    light_distance: f32,
    cutoff_distance: f32,
  ) -> f32 {
    let distance_falloff = 1.0 / max(pow(light_distance, 2.), 0.01);

    // should I use pow2 pow4 for optimization?
    let cutoff = pow(
      clamp(1.0 - pow(light_distance / cutoff_distance, 4.), 0., 1.), // todo use saturate (naga issue)
      2.,
    );

    return distance_falloff * cutoff;
  }
);
