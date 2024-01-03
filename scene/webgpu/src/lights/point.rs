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

impl WebGPULight for IncrementalSignalPtr<PointLight> {
  type Uniform = PointLightShaderInfo;

  fn create_uniform_stream(
    &self,
    ctx: &LightResourceCtx,
    node: Box<dyn Stream<Item = SceneNode> + Unpin>,
  ) -> Box<dyn Stream<Item = Self::Uniform> + Unpin> {
    enum ShaderInfoDelta {
      Position(Vec3<f32>),
      // Shadow(LightShadowAddressInfo),
      Light(Vec3<f32>, f32),
    }

    let derives = ctx.derives.clone();
    let direction = node
      .filter_map_sync(move |node| derives.create_world_matrix_stream(&node))
      .flatten_signal()
      .map(|mat| mat.position())
      .map(ShaderInfoDelta::Position);

    let ill = self
      .single_listen_by(any_change)
      .filter_map_sync(self.defer_weak())
      .map(|light| {
        let light = light.read();
        (
          light.luminance_intensity * light.color_factor,
          light.cutoff_distance,
        )
      })
      .map(|(a, b)| ShaderInfoDelta::Light(a, b));

    let delta = futures::stream_select!(direction, ill);

    Box::new(
      delta.fold_signal(PointLightShaderInfo::default(), |delta, info| {
        match delta {
          ShaderInfoDelta::Position(position) => info.position = position,
          ShaderInfoDelta::Light(i, cutoff_distance) => {
            info.luminance_intensity = i;
            info.cutoff_distance = cutoff_distance;
          }
        };
        Some(*info)
      }),
    )
  }
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
