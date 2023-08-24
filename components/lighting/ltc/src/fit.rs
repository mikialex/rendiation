use rendiation_texture::Size;

use crate::{
  brdf::{Brdf, BrdfEval},
  *,
};

#[derive(Clone, Debug)]
pub struct LtcFitConfig {
  /// width of precomputed square table (theta, alpha)
  pub lut_size: usize,
  /// number of samples used to compute the error during fitting
  pub sample_count: usize,
}

impl LtcFitConfig {
  pub fn lut_data_size(&self) -> usize {
    self.lut_size * self.lut_size
  }

  fn size(&self) -> Size {
    Size::from_usize_pair_min_one((self.lut_size, self.lut_size))
  }
}

impl Default for LtcFitConfig {
  fn default() -> Self {
    Self {
      lut_size: 64,
      sample_count: 32,
    }
  }
}

pub struct LtcFitResult {
  pub ltc_lut1: Texture2DBuffer<Vec4<f32>>,
  pub ltc_lut2: Texture2DBuffer<Vec4<f32>>,
}

pub fn fit(brdf: impl Brdf, config: &LtcFitConfig) -> LtcFitResult {
  let mut tab = vec![Mat3::<f32>::identity(); config.lut_data_size()];
  let mut tab_mag_fresnel = vec![Vec2::<f32>::zero(); config.lut_data_size()];
  let mut tab_sphere = vec![0.; config.lut_data_size()];

  fit_tab(brdf, config, &mut tab, &mut tab_mag_fresnel);
  gen_sphere_tab(config, &mut tab_sphere);
  pack_tab(config, &mut tab, &mut tab_mag_fresnel, &mut tab_sphere)
}

fn fit_tab(
  brdf: impl Brdf,
  config: &LtcFitConfig,
  tab: &mut [Mat3<f32>],
  tab_mag_fresnel: &mut [Vec2<f32>],
) {
  let mut ltc = LTC::default();

  // loop over theta and alpha
  for a_r in 0..config.lut_size {
    let a = config.lut_size - 1 - a_r;
    for t in 0..config.lut_size {
      // parameterized by sqrt(1 - cos(theta))
      let x = t as f32 / (config.lut_size - 1) as f32;
      let ct = 1.0 - x * x;
      let theta = ct.acos().min(f32::PI() / 2.0);
      let v = Vec3::new(theta.sin(), 0., theta.cos());

      // alpha = roughness^2
      let roughness = a as f32 / (config.lut_size - 1) as f32;
      // minimal roughness (avoid singularities)
      let alpha = roughness * roughness.max(0.00001);

      let avg = compute_avg_terms(brdf, v, alpha, config.sample_count);
      ltc.fresnel = avg.fresnel;
      ltc.magnitude = avg.norm;

      // 1. first guess for the fit
      // init the hemisphere in which the distribution is fitted
      // if theta == 0 the lobe is rotationally symmetric and aligned with Z = (0 0 1)
      let isotropic = if t == 0 {
        ltc.x = Vec3::new(1., 0., 0.);
        ltc.y = Vec3::new(0., 1., 0.);
        ltc.z = Vec3::new(0., 0., 1.);

        if a == config.lut_size - 1
        // roughness = 1
        {
          ltc.m11 = 1.0;
          ltc.m22 = 1.0;
        } else {
          // init with roughness of previous fit
          ltc.m11 = tab[a + 1 + t * config.lut_size].a1;
          ltc.m22 = tab[a + 1 + t * config.lut_size].b2;
        }

        ltc.m13 = 0.;
        ltc.update();

        true
      } else {
        // otherwise use previous configuration as first guess
        let l = avg.direction;
        let t1 = Vec3::new(l.z, 0., -l.x);
        let t2 = Vec3::new(0., 1., 0.);
        ltc.x = t1;
        ltc.y = t2;
        ltc.z = l;

        ltc.update();

        false
      };

      // 2. fit (explore parameter space and refine first guess)
      let nm_config = NelderMeadSearchConfig {
        start: vec![ltc.m11, ltc.m22, ltc.m13],
        max_iter: 100,
        delta: 0.05,
        tolerance: 1e-5,
      };
      // Find best-fit LTC lobe (scale, alphax, alphay)
      let (_, best_fit) = nelder_mead(
        |current_fit| {
          update_ltc_fit_result(&mut ltc, current_fit, isotropic);
          compute_error(brdf, &ltc, v, alpha, config.sample_count)
        },
        nm_config,
      );

      update_ltc_fit_result(&mut ltc, &best_fit, isotropic);

      // copy data
      let idx = a + t * config.lut_size;
      tab[idx] = ltc.m;
      tab_mag_fresnel[idx].x = ltc.magnitude;
      tab_mag_fresnel[idx].y = ltc.fresnel;

      // kill useless coefs in matrix
      tab[idx].a2 = 0.;
      tab[idx].b1 = 0.;
      tab[idx].c2 = 0.;
      tab[idx].b3 = 0.;
    }
  }
}

fn gen_sphere_tab(config: &LtcFitConfig, tab_sphere: &mut [f32]) {
  let n = config.lut_size;
  for j in 0..n {
    for i in 0..n {
      let u1 = (i as f32) / (n - 1) as f32;
      let u2 = (j as f32) / (n - 1) as f32;

      // z = cos(elevation angle)
      let z = u1 * 2.0 - 1.0;

      // length of average dir., proportional to sin(sigma)^2
      let len = u2;

      let sigma = len.sqrt().asin();
      let omega = z.acos();

      // compute projected (cosine-weighted) solid angle of spherical cap
      let value = if sigma > 0. {
        ihemi(omega, sigma) / (f32::PI() * len)
      } else {
        z.max(0.)
      };

      if value.is_nan() {
        println!("encounter nan value")
      }

      tab_sphere[i + j * n] = value;
    }
  }
}

fn pack_tab(
  config: &LtcFitConfig,
  tab: &mut [Mat3<f32>],
  tab_mag_fresnel: &mut [Vec2<f32>],
  tab_sphere: &mut [f32],
) -> LtcFitResult {
  let mut ltc_lut1 = Vec::with_capacity(config.lut_data_size());
  let mut ltc_lut2 = Vec::with_capacity(config.lut_data_size());
  for i in 0..config.lut_data_size() {
    let m = tab[i];

    let mut inv_m = m.inverse_or_identity();

    // normalize by the middle element
    inv_m /= inv_m.b2;

    ltc_lut1.push(Vec4::new(inv_m.a1, inv_m.a3, inv_m.c1, inv_m.c3));
    let tab_mag_fresnel = tab_mag_fresnel[i];
    ltc_lut2.push(Vec4::new(
      tab_mag_fresnel.x,
      tab_mag_fresnel.y,
      0.0,
      tab_sphere[i],
    ));
  }

  let ltc_lut1 = Texture2DBuffer::from_raw(ltc_lut1, config.size());
  let ltc_lut2 = Texture2DBuffer::from_raw(ltc_lut2, config.size());

  LtcFitResult { ltc_lut1, ltc_lut2 }
}

#[allow(clippy::upper_case_acronyms)]
struct LTC {
  /// lobe magnitude
  magnitude: f32,
  /// Average Schlick Fresnel term
  fresnel: f32,

  // parametric representation
  m11: f32,
  m22: f32,
  m13: f32,
  x: Vec3<f32>,
  y: Vec3<f32>,
  z: Vec3<f32>,

  // matrix representation
  m: Mat3<f32>,
  inverse_m: Mat3<f32>,
  m_determinant: f32,
}

impl Default for LTC {
  fn default() -> Self {
    let mut r = Self {
      magnitude: Default::default(),
      fresnel: Default::default(),
      m11: 1.,
      m22: 1.,
      m13: 0.,
      x: Vec3::new(1., 0., 0.),
      y: Vec3::new(0., 1., 0.),
      z: Vec3::new(0., 0., 1.),
      m: Mat3::identity(),
      inverse_m: Mat3::identity(),
      m_determinant: 0.,
    };
    r.update();
    r
  }
}

impl LTC {
  fn eval(&self, l: Vec3<f32>) -> f32 {
    let l_original = (self.inverse_m * l).normalize();
    let l_ = self.m * l;

    let length = l_.length();
    let jacobian = self.m_determinant / (length * length * length);

    let d = l_original.z.max(0.) / f32::PI();
    self.magnitude * d / jacobian
  }

  #[rustfmt::skip]
  fn update(&mut self) {
    self.m = Mat3::new(
      self.x.x, self.x.y,self.x.z,
      self.y.x, self.y.y,self.y.z,
      self.z.x, self.z.y,self.z.z,
    ) * Mat3::new(
      self.m11, 0.,       0.,
      0.,       self.m22, 0.,
      self.m13, 0.,       1.
    );
    self.inverse_m = self.m.inverse_or_identity();
    self.m_determinant = self.m.det().abs();
  }

  fn sample(&self, u1: f32, u2: f32) -> NormalizedVec3<f32> {
    let theta = u1.sqrt().acos();
    let phi = u2 * 2. * f32::PI();
    let sample_dir = Vec3::new(
      theta.sin() * phi.cos(),
      theta.sin() * phi.sin(),
      theta.cos(),
    );
    (self.m * sample_dir).into_normalized()
  }
}

#[derive(Default)]
struct AverageInfo {
  /// the average direction of the BRDF
  direction: Vec3<f32>,
  /// the average Schlick Fresnel value
  fresnel: f32,
  /// the norm (albedo) of the BRDF
  norm: f32,
}

fn compute_avg_terms(brdf: impl Brdf, v: Vec3<f32>, alpha: f32, sample: usize) -> AverageInfo {
  let mut avg = AverageInfo::default();

  for j in 0..sample {
    for i in 0..sample {
      let u1 = (i as f32 + 0.5) / sample as f32;
      let u2 = (j as f32 + 0.5) / sample as f32;

      let l = brdf.sample(v, alpha, u1, u2);
      let eval = brdf.eval(v, l, alpha);

      if eval.pdf > 0. {
        let weight = eval.value / eval.pdf;

        let h = (v + l).normalize();

        // accumulate
        avg.norm += weight;
        avg.fresnel += weight * (1.0 - v.dot(h).max(0.0)).powi(5);
        avg.direction += weight * l;
      }
    }
  }

  let sample_count = (sample * sample) as f32;
  avg.norm /= sample_count;
  avg.fresnel /= sample_count;

  // clear y component, which should be zero with isotropic BRDFs
  avg.direction.y = 0.0;
  avg.direction = avg.direction.normalize();

  avg
}

// compute the error between the BRDF and the LTC
// using Multiple Importance Sampling
fn compute_error(brdf: impl Brdf, ltc: &LTC, v: Vec3<f32>, alpha: f32, sample: usize) -> f32 {
  let mut error: f64 = 0.0;

  for j in 0..sample {
    for i in 0..sample {
      let u1 = (i as f32 + 0.5) / sample as f32;
      let u2 = (j as f32 + 0.5) / sample as f32;

      // importance sample LTC
      {
        let l = ltc.sample(u1, u2);
        let BrdfEval {
          value: eval_brdf,
          pdf: pdf_brdf,
        } = brdf.eval(v, *l, alpha);

        let eval_ltc = ltc.eval(*l);
        let pdf_ltc = eval_ltc / ltc.magnitude;

        // error with MIS weight
        let error_ = (eval_brdf - eval_ltc).abs() as f64;
        let error_ = error_ * error_ * error_;
        error += error_ / (pdf_ltc + pdf_brdf) as f64;
      }

      // importance sample BRDF
      {
        let l = brdf.sample(v, alpha, u1, u2);

        let BrdfEval {
          value: eval_brdf,
          pdf: pdf_brdf,
        } = brdf.eval(v, l, alpha);

        let eval_ltc = ltc.eval(l);
        let pdf_ltc = eval_ltc / ltc.magnitude;

        // error with MIS weight
        let error_ = (eval_brdf - eval_ltc).abs() as f64;
        let error_ = error_ * error_ * error_;
        error += error_ / (pdf_ltc + pdf_brdf) as f64;
      }
    }
  }

  (error / (sample * sample) as f64) as f32
}

fn update_ltc_fit_result(ltc: &mut LTC, fit_result: &[f32], isotropic: bool) {
  let m11 = fit_result[0].max(1e-7);
  let m22 = fit_result[1].max(1e-7);
  let m13 = fit_result[2];

  if isotropic {
    ltc.m11 = m11;
    ltc.m22 = m11;
    ltc.m13 = 0.0;
  } else {
    ltc.m11 = m11;
    ltc.m22 = m22;
    ltc.m13 = m13;
  }
  ltc.update();
}

fn sqr(x: f32) -> f32 {
  x * x
}

fn g_f(w: f32, s: f32, g: f32) -> f32 {
  -2.0 * w.sin() * s.cos() * g.cos() + f32::PI() / 2.0 + g.sin() * g.cos()
}

fn h_f(w: f32, s: f32, g: f32) -> f32 {
  let sin_s_sq = sqr(s.sin());
  let cos_g_sq = sqr(g.cos());

  w.cos() * (g.cos() * (sin_s_sq - cos_g_sq).sqrt() + sin_s_sq * (g.cos() / s.sin()).asin())
}

fn ihemi(w: f32, s: f32) -> f32 {
  let g = (s.cos() / w.sin()).asin();
  let sin_s_sq = sqr(s.sin());

  let pi = f32::PI();

  if w >= 0.0 && w <= (pi / 2.0 - s) {
    return pi * w.cos() * sin_s_sq;
  }

  if w >= (pi / 2.0 - s) && w < pi / 2.0 {
    return pi * w.cos() * sin_s_sq + g_f(w, s, g) - h_f(w, s, g);
  }

  if w >= pi / 2.0 && w < (pi / 2.0 + s) {
    return g_f(w, s, g) + h_f(w, s, g);
  }

  0.0
}

struct NelderMeadSearchConfig {
  start: Vec<f32>,
  max_iter: usize,
  delta: f32,
  tolerance: f32,
}

fn mov(r: &mut [f32], v: &[f32]) {
  r.copy_from_slice(v)
}

fn set(r: &mut [f32], v: f32) {
  r.fill(v)
}

fn add(r: &mut [f32], v: &[f32]) {
  r.iter_mut().zip(v.iter()).for_each(|(r, v)| *r += v)
}

/// Downhill simplex solver:
/// http://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method#One_possible_variation_of_the_NM_algorithm
/// using the termination criterion from Numerical Recipes in C++ (3rd Ed.)
#[allow(clippy::needless_range_loop)]
fn nelder_mead(
  mut objective_fn: impl FnMut(&[f32]) -> f32,
  config: NelderMeadSearchConfig,
) -> (f32, Vec<f32>) {
  // standard coefficients from Nelder-Mead
  let reflect = 1.0;
  let expand = 2.0;
  let contract = 0.5;
  let shrink = 0.5;

  //     typedef float point[DIM];
  //     const int NB_POINTS = DIM + 1;

  let number_points = config.start.len() + 1;
  let mut s: Vec<Vec<f32>> = Vec::new();

  // initialize simplex
  s.push(config.start.clone());
  for i in 1..number_points {
    s.push(config.start.clone());
    s[i][i - 1] += config.delta;
  }

  // evaluate function at each point on simplex
  let mut f: Vec<f32> = (0..number_points).map(|i| objective_fn(&s[i])).collect();

  let mut lo = 0;

  let mut o = vec![0.; config.start.len()];
  let mut r = vec![0.; config.start.len()];
  let mut e = vec![0.; config.start.len()];
  let mut c = vec![0.; config.start.len()];
  let len = o.len();
  let len_f = o.len() as f32;

  for _ in 0..config.max_iter {
    // find lowest, highest and next highest
    lo = 0;
    let mut hi = 0;
    let mut nh = 0;
    for i in 1..number_points {
      if f[i] < f[lo] {
        lo = i;
      }
      if f[i] > f[hi] {
        nh = hi;
        hi = i;
      } else if f[i] > f[nh] {
        nh = i;
      }
    }

    // stop if we've reached the required tolerance level
    let a = f[lo].abs();
    let b = f[hi].abs();
    if (a - b).abs() * 2.0 < (a + b) * config.tolerance {
      break;
    }

    // compute centroid (excluding the worst point)
    set(&mut o, 0.);
    for i in 0..number_points {
      if i == hi {
        continue;
      }
      add(&mut o, &s[i])
    }
    o.iter_mut().for_each(|v| *v /= len_f);

    // reflection
    for i in 0..len {
      r[i] = o[i] + reflect * (o[i] - s[hi][i]);
    }

    let fr = objective_fn(&r);
    if fr < f[nh] {
      if fr < f[lo] {
        // expansion
        for i in 0..len {
          e[i] = o[i] + expand * (o[i] - s[hi][i]);
        }

        let fe = objective_fn(&e);
        if fe < fr {
          mov(&mut s[hi], &e);
          f[hi] = fe;
          continue;
        }
      }

      mov(&mut s[hi], &r);
      f[hi] = fr;
      continue;
    }

    // contraction
    for i in 0..len {
      c[i] = o[i] - contract * (o[i] - s[hi][i]);
    }

    let fc = objective_fn(&c);
    if fc < f[hi] {
      mov(&mut s[hi], &c);
      f[hi] = fc;
      continue;
    }

    // reduction
    for k in 0..number_points {
      if k == lo {
        continue;
      }
      for i in 0..len {
        s[k][i] = s[lo][i] + shrink * (s[k][i] - s[lo][i]);
      }
      f[k] = objective_fn(&s[k]);
    }
  }

  // return best point and its value
  (f[lo], s[lo].clone())
}
