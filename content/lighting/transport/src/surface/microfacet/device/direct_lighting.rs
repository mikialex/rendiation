use crate::*;

both!(EmissiveChannel, Vec3<f32>);
both!(SpecularChannel, Vec3<f32>);

// This is the alpha, which is the square of the perceptual roughness
// (perceptual roughness is artist friendly so usually used in material parameters)
both!(RoughnessChannel, f32);
both!(MetallicChannel, f32);
// This is the inverse alpha, also linear
both!(GlossinessChannel, f32);
both!(ReflectanceChannel, f32);

pub struct PhysicalShading;

impl PhysicalShading {
  pub fn construct_shading_impl(builder: &SemanticRegistry) -> ENode<ShaderPhysicalShading> {
    let linear_roughness = builder
      .try_query_fragment_stage::<RoughnessChannel>()
      .or_else(|_| {
        builder
          .try_query_fragment_stage::<GlossinessChannel>()
          .map(|v| val(1.0) - v)
      })
      .unwrap_or_else(|_| val(0.3));

    let base_color = builder
      .try_query_fragment_stage::<ColorChannel>()
      .unwrap_or_else(|_| val(Vec3::splat(0.5)));

    // assume specular workflow
    let (albedo, f0) = if let Ok(specular) = builder.try_query_fragment_stage::<SpecularChannel>() {
      let metallic = specular.max_channel();
      (base_color * (val(1.) - metallic), specular)
    } else {
      // assume metallic workflow
      let metallic = builder
        .try_query_fragment_stage::<MetallicChannel>()
        .unwrap_or_else(|_| val(0.0));

      let reflectance = builder
        .try_query_fragment_stage::<ReflectanceChannel>()
        .unwrap_or_else(|_| val(0.5));

      let dielectric_f0 = compute_dielectric_f0(reflectance);

      let f0 = base_color * metallic + (dielectric_f0 * (val(1.) - metallic)).splat();

      (base_color * (val(1.) - metallic), f0)
    };

    let emissive = builder
      .try_query_fragment_stage::<EmissiveChannel>()
      .unwrap_or_else(|_| val(Vec3::zero()));

    ENode::<ShaderPhysicalShading> {
      albedo,
      f0,
      linear_roughness,
      emissive,
    }
  }
}

pub fn compute_dielectric_f0(reflectance: Node<f32>) -> Node<f32> {
  val(0.16) * reflectance * reflectance
}

impl LightableSurfaceShadingLogicProvider for PhysicalShading {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilder,
  ) -> Box<dyn LightableSurfaceShading> {
    Box::new(PhysicalShading::construct_shading_impl(builder.registry()))
  }
}

/// we have to use the real type name to avoid mysterious impl conflict, what a shame
impl LightableSurfaceShading for ShaderPhysicalShadingShaderAPIInstance {
  fn compute_lighting_by_incident(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    physical_shading_fn(direct_light.construct(), ctx.construct(), self.construct()).expand()
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
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
          specular_and_emissive: shading.emissive,
        })
      });

      let direct_diffuse_brdf = ShaderLambertian {
        albedo: shading.albedo,
      }
      .bsdf(geometry.view_dir, -light.direction, geometry.normal);

      let roughness = shading.linear_roughness;
      let direct_specular_brdf = ShaderSpecular {
        f0: shading.f0,
        normal_distribution_model: ShaderGGX { roughness },
        geometric_shadow_model: ShaderSmithGGXCorrelatedGeometryShadow { roughness },
        fresnel_model: ShaderSchlick,
      }
      .bsdf(geometry.view_dir, -light.direction, geometry.normal);

      cx.do_return(ENode::<ShaderLightingResult> {
        diffuse: light.color * direct_diffuse_brdf * n_dot_l,
        specular_and_emissive: light.color * direct_specular_brdf * n_dot_l + shading.emissive,
      })
    })
    .prepare_parameters()
    .push(light)
    .push(geometry)
    .push(shading)
    .call()
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhysicalShading {
  pub albedo: Vec3<f32>,
  pub linear_roughness: f32,
  pub f0: Vec3<f32>,
  pub emissive: Vec3<f32>,
}

pub fn bias_n_dot_l(n_dot_l: Node<f32>) -> Node<f32> {
  (n_dot_l * val(1.08) - val(0.08)).saturate()
}

use rendiation_shader_library::sampling::hammersley_2d_fn;

/// for ibl
pub fn integrate_brdf(
  roughness: Node<f32>, // perceptual roughness
  n_dot_v: Node<f32>,
  sample_count: Node<u32>,
) -> Node<Vec2<f32>> {
  let roughness2 = roughness * roughness;
  let view = vec3_node(((val(1.) - n_dot_v * n_dot_v).sqrt(), val(0.), n_dot_v));

  let sum = sample_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sample_count);
      let half = hemisphere_importance_sample_dggx(random, roughness2);

      let light = val(2.0) * view.dot(half) * half - view;
      let n_dot_l = light.z().saturate();
      let n_dot_h = half.z().saturate();
      let v_dot_h = view.dot(half).saturate();

      let g = g_smith(n_dot_l, n_dot_v, roughness2);
      let g_vis = (g * v_dot_h / (n_dot_h * n_dot_v)).max(0.);
      let fc = (val(1.) - v_dot_h).pow(5.0);

      vec2_node(((val(1.) - fc) * g_vis, fc * g_vis))
    })
    .sum();

  sum / sample_count.into_f32().splat()
}

pub fn g1_ggx_schlick(n_dot_v: Node<f32>, a: Node<f32>) -> Node<f32> {
  let k = a / val(2.);
  n_dot_v.max(val(0.001)) / (n_dot_v * (val(1.0) - k) + k)
}

pub fn g_smith(n_dot_v: Node<f32>, n_dot_l: Node<f32>, a: Node<f32>) -> Node<f32> {
  g1_ggx_schlick(n_dot_l, a) * g1_ggx_schlick(n_dot_v, a)
}

pub fn hemisphere_importance_sample_dggx(u: Node<Vec2<f32>>, a: Node<f32>) -> Node<Vec3<f32>> {
  let phi = val(2. * f32::PI()) * u.x();
  // NOTE: (aa-1) == (a-1)(a+1) produces better fp accuracy
  let cos_theta2 = (val(1.0) - u.y()) / (val(1.0) + (a + val(1.0)) * (a - val(1.0)) * u.y());
  let cos_theta = cos_theta2.sqrt();
  let sin_theta = (val(1.0) - cos_theta2).sqrt();
  (sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta).into()
}
