use rendiation_algebra::*;
use rendiation_lighting_transport::*;
use rendiation_shader_api::*;

/// Punctual lights are defined as parameterized, infinitely small points that
/// emit light in well-defined directions and intensities.
pub trait PunctualShaderLight {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight>;
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct DirectionalShaderInfo {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

impl PunctualShaderLight for Node<DirectionalShaderInfo> {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    self.expand().compute_incident_light(ctx)
  }
}

impl PunctualShaderLight for ENode<DirectionalShaderInfo> {
  fn compute_incident_light(
    &self,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    ENode::<ShaderIncidentLight> {
      color: self.illuminance,
      direction: self.direction,
    }
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct PointLightShaderInfo {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

impl PunctualShaderLight for Node<PointLightShaderInfo> {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    self.expand().compute_incident_light(ctx)
  }
}

impl PunctualShaderLight for ENode<PointLightShaderInfo> {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - self.position;
    let distance = direction.length();
    let factor = punctual_light_intensity_to_illuminance_factor_fn(distance, self.cutoff_distance);

    ENode::<ShaderIncidentLight> {
      color: self.luminance_intensity * factor,
      direction: direction.normalize(),
    }
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct SpotLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

impl PunctualShaderLight for Node<SpotLightShaderInfo> {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    self.expand().compute_incident_light(ctx)
  }
}

impl PunctualShaderLight for ENode<SpotLightShaderInfo> {
  fn compute_incident_light(
    &self,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let direction = ctx.position - self.position;
    let distance = direction.length();
    let distance_factor =
      punctual_light_intensity_to_illuminance_factor_fn(distance, self.cutoff_distance);

    let direction = direction.normalize();
    let angle_cos = direction.dot(self.direction);
    let angle_factor = angle_cos.smoothstep(self.half_cone_cos, self.half_penumbra_cos);

    let intensity_factor = distance_factor * angle_factor;

    ENode::<ShaderIncidentLight> {
      color: self.luminance_intensity * intensity_factor,
      direction,
    }
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
