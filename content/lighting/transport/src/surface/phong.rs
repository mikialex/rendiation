use crate::*;

both!(ShininessChannel, f32);

pub struct PhongShading;

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhongShading {
  pub diffuse: Vec3<f32>,
  pub shininess: f32,
  pub emissive: Vec3<f32>,
  pub specular: Vec3<f32>,
}

impl LightableSurfaceShadingLogicProvider for PhongShading {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilder,
  ) -> Box<dyn LightableSurfaceShading> {
    let emissive = builder
      .try_query::<EmissiveChannel>()
      .unwrap_or_else(|| val(Vec3::zero()));

    let specular = builder
      .try_query::<SpecularChannel>()
      .unwrap_or_else(|| val(Vec3::one()));

    let shininess = builder
      .try_query::<ShininessChannel>()
      .unwrap_or_else(|| val(0.));

    let base_color = builder
      .try_query::<ColorChannel>()
      .unwrap_or_else(|| val(Vec3::splat(0.5)));

    let shader_ins = ShaderPhongShadingShaderAPIInstance {
      diffuse: base_color,
      shininess,
      emissive,
      specular,
    };
    Box::new(shader_ins)
  }
}

impl LightableSurfaceShading for ShaderPhongShadingShaderAPIInstance {
  fn compute_lighting_by_incident(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let light = -direct_light.direction;
    let half = (light + ctx.view_dir).normalize();
    let n_dot_l = ctx.normal.dot(light).max(0.);
    let n_dot_h = ctx.normal.dot(half).max(0.);

    let specular_ratio = n_dot_h.pow(self.shininess).max(0.);

    ENode::<ShaderLightingResult> {
      diffuse: direct_light.color * self.diffuse * n_dot_l,
      specular_and_emissive: self.emissive + direct_light.color * self.specular * specular_ratio,
    }
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
}
