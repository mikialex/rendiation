//! https://github.com/mrdoob/three.js/blob/dev/examples/jsm/objects/SkyMesh.js#L25
//!
//! the original shader is pretty bad. we should find another more physical sky shader

use crate::*;

pub struct SkyBackgroundParameter {
  pub sun_direction: Vec3<f32>,
  pub luminance: f32,
  pub turbidity: f32,
  pub rayleigh: f32,
  pub mie_coefficient: f32,
  pub mie_directional_g: f32,
}

const UP: Vec3<f32> = Vec3::new(0., 1., 0.);

impl SkyBackgroundParameter {
  pub fn to_uniform(&self) -> SkyBackgroundUniform {
    let sun_direction = self.sun_direction.normalize();

    // earth shadow hack
    let cutoff_angle = f32::PI() / 1.95;
    let steepness = 1.5;
    let ee = 1000.0;

    let zenith_angle_cos = sun_direction.dot(UP);
    let angle = cutoff_angle - zenith_angle_cos.acos();
    let sun_intensity = ee * (1.0 - (angle / steepness).exp()).max(0.);

    let sun_fade = 1.0 - (1.0 - (sun_direction.y / 450000.0).exp()).clamp(0., 1.);

    // wavelength of used primaries, according to preetham
    // let lambda = Vec3::new(680E-9, 550E-9, 450E-9);
    // this pre-calculation replaces older TotalRayleigh(vec3 lambda) function:
    // (8.0 * pow(pi, 3.0) * pow(pow(n, 2.0) - 1.0, 2.0) * (6.0 + 3.0 * pn)) / (3.0 * N * pow(lambda, vec3(4.0)) * (6.0 - 7.0 * pn))
    let total_rayleigh = Vec3::new(
      5.804542996261093E-6,
      1.3562911419845635E-5,
      3.0265902468824876E-5,
    );

    let rayleigh_coefficient = self.rayleigh - (1.0 * (1.0 - sun_fade));
    let beta_r = total_rayleigh * rayleigh_coefficient;

    // mie stuff
    // K coefficient for the primaries
    // let v = 4.0;
    // let k = Vec3::new(0.686, 0.678, 0.666);
    // MieConst = pi * pow((2.0 * pi) / lambda, vec3(v - 2.0)) * k
    let mie_const = Vec3::new(
      1.8399918514433978E14,
      2.7798023919660528E14,
      4.0790479543861094E14,
    );

    let total_mie = (0.2 * self.turbidity) * 10E-18 * 0.434 * mie_const;
    let beta_m = total_mie * self.mie_coefficient;

    SkyBackgroundUniform {
      beta_r,
      beta_m,
      sun_direction,
      sun_intensity,
      sun_fade,
      mie_directional_g: self.mie_directional_g,
      luminance: self.luminance,
    }
  }
}

impl Default for SkyBackgroundParameter {
  fn default() -> Self {
    Self {
      sun_direction: Vec3::one().normalize(),
      luminance: 0.3,
      turbidity: 1.0,
      rayleigh: 1.0,
      mie_coefficient: 0.003,
      mie_directional_g: 0.8,
    }
  }
}

#[derive(Debug, Copy, Clone, ShaderStruct)]
pub struct SkyBackgroundUniform {
  pub beta_r: Vec3<f32>,
  pub beta_m: Vec3<f32>,
  pub mie_directional_g: f32,
  pub sun_direction: Vec3<f32>,
  pub sun_intensity: f32,
  pub sun_fade: f32,
  pub luminance: f32,
}

/// return hdr color
pub fn shade_sky(sky: Node<SkyBackgroundUniform>, direction: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let sky = sky.expand();

  let rayleigh_zenith_length = 8.4E3;
  let mie_zenith_length = 1.25E3;

  // optical length
  // cutoff angle at 90 to avoid singularity in next formula.
  let zenith_angle = direction.dot(val(UP)).max(0.).acos();
  let inverse = val(1.0)
    / (zenith_angle.cos()
      + val(0.15) * (val(93.885) - ((zenith_angle * val(180.0)) / val(f32::PI()))).pow(-1.253));
  let s_r = val(rayleigh_zenith_length) * inverse;
  let s_r = s_r.splat::<Vec3<_>>();
  let s_m = val(mie_zenith_length) * inverse;
  let s_m = s_m.splat::<Vec3<_>>();

  // combined extinction factor
  let fex = (-(sky.beta_r * s_r + sky.beta_m * s_m)).exp();

  // in scattering
  let cos_theta = sky.sun_direction.dot(direction);

  // 3.0 / (16.0 * pi)
  const THREE_OVER_SIXTEEN_PI: f32 = 0.05968310365946075;
  // 1.0 / (4.0 * pi)
  const ONE_OVER_FOUR_PI: f32 = 0.07957747154594767;
  fn rayleigh_phase(cos_theta: Node<f32>) -> Node<f32> {
    val(THREE_OVER_SIXTEEN_PI) * (val(1.0) + cos_theta * cos_theta)
  }

  fn hg_phase(cos_theta: Node<f32>, g: Node<f32>) -> Node<f32> {
    let g2 = g * g;
    let inverse = val(1.0) / (val(1.0) - val(2.0) * g * cos_theta + g2).pow(1.5);
    val(ONE_OVER_FOUR_PI) * ((val(1.0) - g2) * inverse)
  }

  let r_phase = rayleigh_phase(cos_theta * val(0.5) + val(0.5));
  let beta_r_theta = sky.beta_r * r_phase;

  let m_phase = hg_phase(cos_theta, sky.mie_directional_g);
  let beta_m_theta = sky.beta_m * m_phase;

  let intensity_base =
    sky.sun_intensity * ((beta_r_theta + beta_m_theta) / (sky.beta_r + sky.beta_m));
  let intensity_factor = intensity_base * (val(Vec3::splat(1.0)) - fex);

  let lin = intensity_factor.pow(val(Vec3::splat(1.5)));
  let lin_mix_base = (val(1.0) - sky.sun_direction.dot(val(UP)))
    .pow(5.0)
    .saturate();
  let lin_mix = lin_mix_base.mix(
    val(Vec3::splat(1.)),
    (intensity_base * fex).pow(Vec3::splat(0.5)),
  );

  let lin = lin * lin_mix;

  // night sky
  let l0 = val(Vec3::splat(0.1)) * fex;

  // composition + solar disc
  // 66 arc seconds -> degrees, and the cosine of that
  let sun_angular_diameter_cos = val(0.999956676946448443553574619906976478926848692873900859324);

  let sun_disk = cos_theta.smoothstep(
    sun_angular_diameter_cos,
    sun_angular_diameter_cos + val(0.00002),
  );
  let l0 = l0 + (sky.sun_intensity * val(Vec3::splat(19000.0)) * fex) * sun_disk;

  let tex_color = (lin + l0) * val(Vec3::splat(0.04)) + val(Vec3::new(0.0, 0.0003, 0.00075));
  let hdr = (val(2.0) / sky.luminance.pow(4.0)).log2() * tex_color;

  //   let curr = Uncharted2Tonemap(hdr);
  //   let color = curr * whiteScale;

  //   let ret_color = pow(color, vec3(1.0 / (1.2 + (1.2 * sky.sun_fade))));

  let ret_color = hdr;

  #[allow(clippy::let_and_return)]
  ret_color
}
