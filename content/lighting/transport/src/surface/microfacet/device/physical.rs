use crate::*;

both!(EmissiveChannel, Vec3<f32>);
both!(SpecularChannel, Vec3<f32>);

// This is the alpha, which is the square of the perceptual roughness
// (perceptual roughness is artist friendly so usually used in material parameters)
both!(RoughnessChannel, f32);
both!(MetallicChannel, f32);
both!(GlossinessChannel, f32);
both!(ReflectanceChannel, f32);

pub struct PhysicalShading;

pub fn compute_dielectric_f0(reflectance: Node<f32>) -> Node<f32> {
  val(0.16) * reflectance * reflectance
}

impl PhysicalShading {
  pub fn construct_shading_impl(builder: &SemanticRegistry) -> ENode<ShaderPhysicalShading> {
    let perceptual_roughness = builder
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
    let (diffuse, f0) = if let Ok(specular) = builder.try_query_fragment_stage::<SpecularChannel>()
    {
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
      diffuse,
      f0,
      perceptual_roughness,
      emissive,
    }
  }
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
        diffuse: light.color * direct_diffuse_brdf * n_dot_l + shading.emissive,
        specular: light.color * direct_specular_brdf * n_dot_l,
      })
    })
    .prepare_parameters()
    .push(light)
    .push(geometry)
    .push(shading)
    .call()
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhysicalShading {
  pub diffuse: Vec3<f32>,
  pub perceptual_roughness: f32,
  pub f0: Vec3<f32>,
  pub emissive: Vec3<f32>,
}

pub fn bias_n_dot_l(n_dot_l: Node<f32>) -> Node<f32> {
  (n_dot_l * val(1.08) - val(0.08)).saturate()
}

/// Microfacet Models for Refraction through Rough Surfaces - equation (33)
/// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
pub fn d_ggx(n_o_h: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
  let d = (n_o_h * roughness4 - n_o_h) * n_o_h + val(1.0);
  roughness4 / (val(f32::PI()) * d * d)
}

// NOTE: Basically same as
// https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// However, calculate a F90 instead of using 1.0 directly
#[shader_fn]
fn fresnel(v_dot_h: Node<f32>, f0: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let fc = (val(1.0) - v_dot_h).pow(5.0);
  let f90 = (f0 * val(50.0)).clamp(Vec3::zero(), Vec3::one());
  f90 * fc + f0 * (val(1.0) - fc)
}

/// Moving Frostbite to Physically Based Rendering 3.0 - page 12, listing 2
/// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
fn v_smith_correlated(n_dot_l: Node<f32>, n_dot_v: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
  let vis_smith_v = n_dot_v * (n_dot_v * (n_dot_v - n_dot_v * roughness4) + roughness4).sqrt();
  let vis_smith_l = n_dot_l * (n_dot_l * (n_dot_l - n_dot_l * roughness4) + roughness4).sqrt();
  val(0.5) / (vis_smith_v + vis_smith_l)
}

pub fn evaluate_brdf_diffuse(diffuse_color: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  val(1. / f32::PI()) * diffuse_color
}

pub fn evaluate_brdf_specular(
  shading: ENode<ShaderPhysicalShading>,
  v: Node<Vec3<f32>>,
  l: Node<Vec3<f32>>,
  n: Node<Vec3<f32>>,
) -> Node<Vec3<f32>> {
  const EPSILON_SHADING: f32 = 0.0001;

  let h = (l + v).normalize();
  let n_dot_l = l.dot(n).max(0.0);
  let n_dot_v = n.dot(v).max(EPSILON_SHADING);
  let n_dot_h = n.dot(h).max(EPSILON_SHADING);
  let v_hot_h = v.dot(h).max(EPSILON_SHADING);

  let roughness2 = shading.perceptual_roughness;
  let roughness4 = roughness2 * roughness2;

  let f = fresnel(v_hot_h, shading.f0);
  let d = d_ggx(n_dot_h, roughness4).max(0.0);
  let g = v_smith_correlated(n_dot_l, n_dot_v, roughness4).max(0.0);

  d * g * f
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
