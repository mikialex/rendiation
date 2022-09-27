use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub cutoff_distance: f32,
  pub node_info: TransformGPUData, // maybe used later in shadowmap
}

impl ShaderLight for PointLightShaderInfo {
  fn name() -> &'static str {
    "point_light"
  }
  type Dependency = ();
  fn create_dep(_: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency {}
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    _dep: &Self::Dependency,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight> {
    let direction = ctx.position - light.node_info.expand().world_matrix.position();
    let distance = direction.length();
    let factor = punctual_light_intensity_to_irradiance_factor(distance, light.cutoff_distance);

    ExpandedNode::<ShaderIncidentLight> {
      color: light.intensity * factor,
      direction: direction.normalize(),
    }
  }
}

impl WebGPUSceneLight for PointLight {
  fn collect(&self, sys: &mut ForwardLightingSystem, node: &SceneNode) {
    let lights = sys
      .lights_collections
      .entry(self.type_id())
      .or_insert_with(|| Box::new(LightList::<PointLightShaderInfo>::default()));
    let lights = lights
      .as_any_mut()
      .downcast_mut::<LightList<PointLightShaderInfo>>()
      .unwrap();

    let gpu = PointLightShaderInfo {
      intensity: self.intensity,
      cutoff_distance: self.cutoff_distance,
      node_info: TransformGPUData::from_node(node, None),
      ..Zeroable::zeroed()
    };

    lights.lights.push(gpu)
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
