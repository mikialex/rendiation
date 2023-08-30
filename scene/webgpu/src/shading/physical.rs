use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhysicalShading {
  pub diffuse: Vec3<f32>,
  pub perceptual_roughness: f32,
  pub f0: Vec3<f32>,
}

both!(EmissiveChannel, Vec3<f32>);

both!(AlphaCutChannel, f32);
both!(AlphaChannel, f32);
both!(ColorChannel, Vec3<f32>);
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

impl LightableSurfaceShading for PhysicalShading {
  type ShaderStruct = ShaderPhysicalShading;
  fn construct_shading(builder: &mut ShaderFragmentBuilder) -> ENode<Self::ShaderStruct> {
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

      (base_color, f0)
    };

    ENode::<Self::ShaderStruct> {
      diffuse,
      f0,
      perceptual_roughness,
    }
  }

  fn compute_lighting_by_incident(
    self_node: &ENode<Self::ShaderStruct>,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    physical_shading_fn(
      direct_light.construct(),
      ctx.construct(),
      self_node.construct(),
    )
    .expand()
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

fn bias_n_dot_l(n_dot_l: Node<f32>) -> Node<f32> {
  (n_dot_l * val(1.08) - val(0.08)).saturate()
}

/// Microfacet Models for Refraction through Rough Surfaces - equation (33)
/// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
fn d_ggx(n_o_h: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
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
fn v_ggx_smith_correlated(
  n_dot_l: Node<f32>,
  n_dot_v: Node<f32>,
  roughness4: Node<f32>,
) -> Node<f32> {
  let vis_smith_v = n_dot_v * (n_dot_v * (n_dot_v - n_dot_v * roughness4) + roughness4).sqrt();
  let vis_smith_l = n_dot_l * (n_dot_l * (n_dot_l - n_dot_l * roughness4) + roughness4).sqrt();
  val(0.5) / (vis_smith_v + vis_smith_l)
}

fn evaluate_brdf_diffuse(diffuse_color: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  val(1. / f32::PI()) * diffuse_color
}

fn evaluate_brdf_specular(
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
  let g = v_ggx_smith_correlated(n_dot_l, n_dot_v, roughness4).max(0.0);

  d * g * f
}
