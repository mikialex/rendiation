use crate::*;

both!(EmissiveChannel, Vec3<f32>);

both!(AlphaCutChannel, f32);
both!(AlphaChannel, f32);
both!(SpecularChannel, Vec3<f32>);

// This is the alpha, which is the square of the perceptual roughness
// (perceptual roughness is artist friendly so usually used in material parameters)
both!(RoughnessChannel, f32);
both!(MetallicChannel, f32);
both!(GlossinessChannel, f32);
both!(ReflectanceChannel, f32);

pub struct PhysicalShading;
pub struct ShadingSelection;

pub fn compute_dielectric_f0(reflectance: Node<f32>) -> Node<f32> {
  val(0.16) * reflectance * reflectance
}

impl LightableSurfaceShadingProvider for PhysicalShading {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilder,
  ) -> Box<dyn LightableSurfaceShading> {
    let perceptual_roughness = builder
      .query::<RoughnessChannel>()
      .or_else(|_| Ok(val(1.) - builder.query::<GlossinessChannel>()?))
      .unwrap_or_else(|_: ShaderBuildError| val(0.3));

    let base_color = builder
      .query::<ColorChannel>()
      .unwrap_or_else(|_| val(Vec3::splat(0.5)));

    // assume specular workflow
    let (diffuse, f0) = if let Ok(specular) = builder.query::<SpecularChannel>() {
      let metallic = specular.max_channel();
      (base_color * (val(1.) - metallic), specular)
    } else {
      // assume metallic workflow
      let metallic = builder
        .query::<MetallicChannel>()
        .unwrap_or_else(|_| val(0.0));

      let reflectance = builder
        .query::<ReflectanceChannel>()
        .unwrap_or_else(|_| val(0.5));

      let dielectric_f0 = compute_dielectric_f0(reflectance);

      let f0 = base_color * metallic + (dielectric_f0 * (val(1.) - metallic)).splat();

      (base_color * (val(1.) - metallic), f0)
    };

    Box::new(ENode::<ShaderPhysicalShading> {
      diffuse,
      f0,
      perceptual_roughness,
    })
  }
}

impl LightableSurfaceShading for ENode<ShaderPhysicalShading> {
  fn compute_lighting_by_incident_dyn(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    physical_shading_fn(direct_light.construct(), ctx.construct(), self.construct()).expand()
  }
}

fn physical_shading_fn(
  light: Node<ShaderIncidentLight>,
  geometry: Node<ShaderLightingGeometricCtx>,
  shading: Node<ShaderPhysicalShading>,
) -> Node<ShaderLightingResult> {
  get_shader_fn::<ShaderLightingResult>(shader_fn_name(physical_shading_fn))
    .or_define(|cx| {
      let light = cx.push_fn_parameter_by(light).expand();
      let geometry = cx.push_fn_parameter_by(geometry).expand();
      let shading = cx.push_fn_parameter_by(shading).expand();

      let n_dot_l = bias_n_dot_l((-light.direction).dot(geometry.normal));

      if_by(n_dot_l.equals(0.), || {
        cx.do_return(ENode::<ShaderLightingResult> {
          diffuse: val(Vec3::zero()),
          specular: val(Vec3::zero()),
        })
      });

      let direct_diffuse_brdf = evaluate_brdf_diffuse(shading.diffuse);
      let direct_specular_brdf = evaluate_brdf_specular(
        shading,
        geometry.view_dir,
        -light.direction,
        geometry.normal,
      );

      cx.do_return(ENode::<ShaderLightingResult> {
        diffuse: light.color * direct_diffuse_brdf * n_dot_l,
        specular: light.color * direct_specular_brdf * n_dot_l,
      })
    })
    .prepare_parameters()
    .push(light)
    .push(geometry)
    .push(shading)
    .call()
}
