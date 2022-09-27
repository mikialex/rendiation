use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
  pub node_info: TransformGPUData, // maybe used later in shadowmap
}

impl ShaderLight for SpotLightShaderInfo {
  fn name() -> &'static str {
    "spot_light"
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
    let distance_factor =
      punctual_light_intensity_to_irradiance_factor(distance, light.cutoff_distance);

    // let angle_cos = direction.dot(light.direction);
    // let angle_factor = smoothstep(light.half_cone_cos, light.half_penumbra_cos, angle_cos);

    todo!()

    // ExpandedNode::<ShaderIncidentLight> {
    //   color: light.intensity * distance_factor * angle_factor,
    //   direction: direction.normalize(),
    // }
  }
}

impl WebGPUSceneLight for SpotLight {
  fn collect(&self, sys: &mut ForwardLightingSystem, node: &SceneNode) {
    let lights = sys
      .lights_collections
      .entry(self.type_id())
      .or_insert_with(|| Box::new(LightList::<SpotLightShaderInfo>::default()));
    let lights = lights
      .as_any_mut()
      .downcast_mut::<LightList<SpotLightShaderInfo>>()
      .unwrap();

    let gpu = SpotLightShaderInfo {
      intensity: self.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      cutoff_distance: self.cutoff_distance,
      half_cone_cos: self.half_cone_angle.cos(),
      half_penumbra_cos: self.half_penumbra_angle.cos(),
      node_info: TransformGPUData::from_node(node, None),
      ..Zeroable::zeroed()
    };

    lights.lights.push(gpu)
  }
}
