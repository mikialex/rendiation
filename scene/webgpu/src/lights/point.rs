use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

impl PunctualShaderLight for PointLightShaderInfo {
  fn compute_incident_light(
    _: &ShaderFragmentBuilderView,
    light: &ENode<Self>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - light.position;
    let distance = direction.length();
    let factor = punctual_light_intensity_to_illuminance_factor_fn(distance, light.cutoff_distance);

    ENode::<ShaderIncidentLight> {
      color: light.luminance_intensity * factor,
      direction: direction.normalize(),
    }
  }
}

fn point_lights_position(
  node_mats: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  light_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<PointLight>>,
) -> impl ReactiveCollection<AllocIdx<PointLight>, Vec3<f32>> {
  // this is costly because the light count is always very small in a scene
  node_mats
    .one_to_many_fanout(light_node_relation)
    .collective_map(|mat| mat.position())
}

fn point_lights_intensity() -> impl ReactiveCollection<AllocIdx<PointLight>, Vec3<f32>> {
  storage_of::<PointLight>()
    .listen_all_instance_changed_set()
    .collective_execute_map_by(|| {
      let compute = storage_of::<PointLight>()
        .create_key_mapper(|light, _| light.luminance_intensity * light.color_factor);
      move |k, _| compute(*k)
    })
}

fn point_lights_uniform(
  intensity: impl ReactiveCollection<AllocIdx<PointLight>, Vec3<f32>>,
  directions: impl ReactiveCollection<AllocIdx<PointLight>, Vec3<f32>>,
) -> impl ReactiveCollection<AllocIdx<PointLight>, SpotLightShaderInfo> {
  intensity
    .collective_zip(directions)
    .collective_map(|(intensity, direction)| SpotLightShaderInfo {
      luminance_intensity: intensity,
      direction,
      ..Default::default()
    })
}

/// based upon Frostbite 3 Moving to Physically-based Rendering
/// page 32, equation 26: E[window1]
/// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
/// this is intended to be used on spot and point lights who are represented as luminous intensity
/// but who must be converted to illuminance for surface lighting calculation
#[shader_fn]
pub fn punctual_light_intensity_to_illuminance_factor(
  light_distance: Node<f32>,
  cutoff_distance: Node<f32>,
) -> Node<f32> {
  let l2 = light_distance * light_distance;
  let distance_falloff = val(1.0) / l2.max(0.01);

  let ratio = light_distance / cutoff_distance;
  let cutoff = val(1.0) - ratio * ratio * ratio * ratio;
  let cutoff = cutoff.saturate();
  let cutoff = cutoff * cutoff;

  distance_falloff * cutoff
}
