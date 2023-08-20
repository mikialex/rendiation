use crate::{
  brdf::{Brdf, BrdfEval},
  *,
};

#[derive(Clone, Debug)]
pub struct LtcFitConfig {
  /// size of precomputed table (theta, alpha)
  pub lut_size: usize,
  /// number of samples used to compute the error during fitting
  pub sample_count: usize,
  /// minimal roughness (avoid singularities)
  pub minimal_roughness: f32,
}

impl Default for LtcFitConfig {
  fn default() -> Self {
    Self {
      lut_size: 64,
      sample_count: 32,
      minimal_roughness: 0.00001,
    }
  }
}

pub struct LtcFitResult {
  pub ltc_lut1: Texture2DBuffer<Vec4<f32>>,
  pub ltc_lut2: Texture2DBuffer<Vec4<f32>>,
}

pub fn fit(config: &LtcFitConfig) -> LtcFitResult {
  let mut tab = vec![Mat3::<f32>::identity(); config.lut_size * config.lut_size];
  let mut tab_mag_fresnel = vec![Vec2::<f32>::zero(); config.lut_size * config.lut_size];
  let mut tab_sphere = vec![0.; config.lut_size * config.lut_size];

  fit_tab(config, &mut tab, &mut tab_mag_fresnel);
  gen_sphere_tab(config, &mut tab_sphere);
  pack_tab(config, &mut tab, &mut tab_mag_fresnel, &mut tab_sphere)
}

fn fit_tab(config: &LtcFitConfig, tab: &mut Vec<Mat3<f32>>, tab_mag_fresnel: &mut Vec<Vec2<f32>>) {
  todo!()
}

fn gen_sphere_tab(config: &LtcFitConfig, tab_sphere: &mut Vec<f32>) {
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
        //             value = ihemi(omega, sigma)/(pi*len);
        todo!()
      } else {
        z.max(0.)
      };
      let value = 0.0;

      if value.is_nan() {
        println!("encounter nan value")
      }

      tab_sphere[i + j * n] = value;
    }
  }
}

fn pack_tab(
  config: &LtcFitConfig,
  tab: &mut Vec<Mat3<f32>>,
  tab_mag_fresnel: &mut Vec<Vec2<f32>>,
  tab_sphere: &mut Vec<f32>,
) -> LtcFitResult {
  // for (int i = 0; i < N*N; ++i)
  // {
  //     const mat3& m = tab[i];

  //     mat3 invM = inverse(m);

  //     // normalize by the middle element
  //     invM /= invM[1][1];

  //     // store the variable terms
  //     tex1[i].x = invM[0][0];
  //     tex1[i].y = invM[0][2];
  //     tex1[i].z = invM[2][0];
  //     tex1[i].w = invM[2][2];

  //     tex2[i].x = tabMagFresnel[i][0];
  //     tex2[i].y = tabMagFresnel[i][1];
  //     tex2[i].z = 0.0f; // unused
  //     tex2[i].w = tabSphere[i];
  // }
  todo!()
}

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

fn compute_avg_terms<B: Brdf>(v: Vec3<f32>, alpha: f32, sample: usize) -> AverageInfo {
  let mut avg = AverageInfo::default();

  for j in 0..sample {
    for i in 0..sample {
      let u1 = (i as f32 + 0.5) / sample as f32;
      let u2 = (j as f32 + 0.5) / sample as f32;

      let l = B::sample(v, alpha, u1, u2);
      let eval = B::eval(v, l, alpha);

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
fn compute_error<B: Brdf>(ltc: &LTC, v: Vec3<f32>, alpha: f32, sample: usize) -> f32 {
  let mut error: f64 = 0.0;

  for j in 0..sample {
    for i in 0..sample {
      let u1 = (i as f32 + 0.5) / sample as f32;
      let u2 = (j as f32 + 0.5) / sample as f32;

      // importance sample LTC
      {
        // sample
        let l = ltc.sample(u1, u2);
        let BrdfEval {
          value: eval_brdf,
          pdf: pdf_brdf,
        } = B::eval(v, *l, alpha);

        let eval_ltc = ltc.eval(*l);
        let pdf_ltc = eval_ltc / ltc.magnitude;

        // error with MIS weight
        let error_ = (eval_brdf - eval_ltc).abs() as f64;
        let error_ = error_ * error_ * error_;
        error += error_ / (pdf_ltc + pdf_brdf) as f64;
      }

      // importance sample BRDF
      {
        // sample
        let l = B::sample(v, alpha, u1, u2);

        let BrdfEval {
          value: eval_brdf,
          pdf: pdf_brdf,
        } = B::eval(v, l, alpha);

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

// struct FitLTC
// {
//     FitLTC(LTC& ltc_, const Brdf& brdf, bool isotropic_, const vec3& V_, float alpha_) :
//         ltc(ltc_), brdf(brdf), V(V_), alpha(alpha_), isotropic(isotropic_)
//     {
//     }

//     void update(const float* params)
//     {
//         float m11 = std::max<float>(params[0], 1e-7f);
//         float m22 = std::max<float>(params[1], 1e-7f);
//         float m13 = params[2];

//         if (isotropic)
//         {
//             ltc.m11 = m11;
//             ltc.m22 = m11;
//             ltc.m13 = 0.0f;
//         }
//         else
//         {
//             ltc.m11 = m11;
//             ltc.m22 = m22;
//             ltc.m13 = m13;
//         }
//         ltc.update();
//     }

//     float operator()(const float* params)
//     {
//         update(params);
//         return computeError(ltc, brdf, V, alpha);
//     }

//     const Brdf& brdf;
//     LTC& ltc;
//     bool isotropic;

//     const vec3& V;
//     float alpha;
// };

// // fit brute force
// // refine first guess by exploring parameter space
// void fit(LTC& ltc, const Brdf& brdf, const vec3& V, const float alpha, const float epsilon =
// 0.05f, const bool isotropic = false) {
//     float startFit[3] = { ltc.m11, ltc.m22, ltc.m13 };
//     float resultFit[3];

//     FitLTC fitter(ltc, brdf, isotropic, V, alpha);

//     // Find best-fit LTC lobe (scale, alphax, alphay)
//     float error = NelderMead<3>(resultFit, startFit, epsilon, 1e-5f, 100, fitter);

//     // Update LTC with best fitting values
//     fitter.update(resultFit);
// }

// // fit data
// void fitTab(mat3* tab, vec2* tabMagFresnel, const int N, const Brdf& brdf)
// {
//     LTC ltc;

//     // loop over theta and alpha
//     for (int a = N - 1; a >=     0; --a)
//     for (int t =     0; t <= N - 1; ++t)
//     {
//         // parameterised by sqrt(1 - cos(theta))
//         float x = t/float(N - 1);
//         float ct = 1.0f - x*x;
//         float theta = std::min<float>(1.57f, acosf(ct));
//         const vec3 V = vec3(sinf(theta), 0, cosf(theta));

//         // alpha = roughness^2
//         float roughness = a/float(N - 1);
//         float alpha = std::max<float>(roughness*roughness, MIN_ALPHA);

//         cout << "a = " << a << "\t t = " << t  << endl;
//         cout << "alpha = " << alpha << "\t theta = " << theta << endl;
//         cout << endl;

//         vec3 averageDir;
//         computeAvgTerms(brdf, V, alpha, ltc.magnitude, ltc.fresnel, averageDir);

//         bool isotropic;

//         // 1. first guess for the fit
//         // init the hemisphere in which the distribution is fitted
//         // if theta == 0 the lobe is rotationally symmetric and aligned with Z = (0 0 1)
//         if (t == 0)
//         {
//             ltc.X = vec3(1, 0, 0);
//             ltc.Y = vec3(0, 1, 0);
//             ltc.Z = vec3(0, 0, 1);

//             if (a == N - 1) // roughness = 1
//             {
//                 ltc.m11 = 1.0f;
//                 ltc.m22 = 1.0f;
//             }
//             else // init with roughness of previous fit
//             {
//                 ltc.m11 = tab[a + 1 + t*N][0][0];
//                 ltc.m22 = tab[a + 1 + t*N][1][1];
//             }

//             ltc.m13 = 0;
//             ltc.update();

//             isotropic = true;
//         }
//         // otherwise use previous configuration as first guess
//         else
//         {
//             vec3 L = averageDir;
//             vec3 T1(L.z, 0, -L.x);
//             vec3 T2(0, 1, 0);
//             ltc.X = T1;
//             ltc.Y = T2;
//             ltc.Z = L;

//             ltc.update();

//             isotropic = false;
//         }

//         // 2. fit (explore parameter space and refine first guess)
//         float epsilon = 0.05f;
//         fit(ltc, brdf, V, alpha, epsilon, isotropic);

//         // copy data
//         tab[a + t*N] = ltc.M;
//         tabMagFresnel[a + t*N][0] = ltc.magnitude;
//         tabMagFresnel[a + t*N][1] = ltc.fresnel;

//         // kill useless coefs in matrix
//         tab[a+t*N][0][1] = 0;
//         tab[a+t*N][1][0] = 0;
//         tab[a+t*N][2][1] = 0;
//         tab[a+t*N][1][2] = 0;

//         cout << tab[a+t*N][0][0] << "\t " << tab[a+t*N][1][0] << "\t " << tab[a+t*N][2][0] <<
// endl;         cout << tab[a+t*N][0][1] << "\t " << tab[a+t*N][1][1] << "\t " << tab[a+t*N][2][1]
// << endl;         cout << tab[a+t*N][0][2] << "\t " << tab[a+t*N][1][2] << "\t " <<
// tab[a+t*N][2][2] << endl;         cout << endl;
//     }
// }

fn sqr(x: f32) -> f32 {
  x * x
}

fn G(w: f32, s: f32, g: f32) -> f32 {
  -2.0 * w.sin() * s.cos() * g.cos() + f32::PI() / 2.0 + g.sin() * g.cos()
}

fn H(w: f32, s: f32, g: f32) -> f32 {
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
    return pi * w.cos() * sin_s_sq + G(w, s, g) - H(w, s, g);
  }

  if w >= pi / 2.0 && w < (pi / 2.0 + s) {
    return G(w, s, g) + H(w, s, g);
  }

  0.0
}

fn mov<const N: usize>(r: &mut [f32; N], v: &[f32; N]) {
  *r = *v;
}

fn set<const N: usize>(r: &mut [f32; N], v: f32) {
  *r = [v; N];
}

fn add<const N: usize>(r: &mut [f32; N], v: &[f32; N]) {
  r.iter_mut().zip(v.iter()).for_each(|(r, v)| *r += v)
}

struct NelderMeadSearchConfig<const N: usize> {
  start: [f32; N],
  max_iter: usize,
  delta: f32,
  pmin: f32,
  tolerance: f32,
}

/// Downhill simplex solver:
/// http://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method#One_possible_variation_of_the_NM_algorithm
/// using the termination criterion from Numerical Recipes in C++ (3rd Ed.)
fn nelder_mead<const N: usize>(
  objective_fn: impl Fn(&[f32; N]) -> f32,
  config: NelderMeadSearchConfig<N>,
) -> [f32; N] {
  todo!()
}

// // Downhill simplex solver:
// // http://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method#One_possible_variation_of_the_NM_algorithm
// // using the termination criterion from Numerical Recipes in C++ (3rd Ed.)
// template<int DIM, typename FUNC>
// float NelderMead(
//     float* pmin, const float* start, float delta, float tolerance, int maxIters, FUNC
// objectiveFn) {
//     // standard coefficients from Nelder-Mead
//     const float reflect  = 1.0f;
//     const float expand   = 2.0f;
//     const float contract = 0.5f;
//     const float shrink   = 0.5f;

//     typedef float point[DIM];
//     const int NB_POINTS = DIM + 1;

//     point s[NB_POINTS];
//     float f[NB_POINTS];

//     // initialise simplex
//     mov(s[0], start, DIM);
//     for (int i = 1; i < NB_POINTS; i++)
//     {
//         mov(s[i], start, DIM);
//         s[i][i - 1] += delta;
//     }

//     // evaluate function at each point on simplex
//     for (int i = 0; i < NB_POINTS; i++)
//         f[i] = objectiveFn(s[i]);

//     int lo = 0, hi, nh;

//     for (int j = 0; j < maxIters; j++)
//     {
//         // find lowest, highest and next highest
//         lo = hi = nh = 0;
//         for (int i = 1; i < NB_POINTS; i++)
//         {
//             if (f[i] < f[lo])
//                 lo = i;
//             if (f[i] > f[hi])
//             {
//                 nh = hi;
//                 hi = i;
//             }
//             else if (f[i] > f[nh])
//                 nh = i;
//         }

//         // stop if we've reached the required tolerance level
//         float a = fabsf(f[lo]);
//         float b = fabsf(f[hi]);
//         if (2.0f*fabsf(a - b) < (a + b)*tolerance)
//             break;

//         // compute centroid (excluding the worst point)
//         point o;
//         set(o, 0.0f, DIM);
//         for (int i = 0; i < NB_POINTS; i++)
//         {
//             if (i == hi) continue;
//             add(o, s[i], DIM);
//         }

//         for (int i = 0; i < DIM; i++)
//             o[i] /= DIM;

//         // reflection
//         point r;
//         for (int i = 0; i < DIM; i++)
//             r[i] = o[i] + reflect*(o[i] - s[hi][i]);

//         float fr = objectiveFn(r);
//         if (fr < f[nh])
//         {
//             if (fr < f[lo])
//             {
//                 // expansion
//                 point e;
//                 for (int i = 0; i < DIM; i++)
//                     e[i] = o[i] + expand*(o[i] - s[hi][i]);

//                 float fe = objectiveFn(e);
//                 if (fe < fr)
//                 {
//                     mov(s[hi], e, DIM);
//                     f[hi] = fe;
//                     continue;
//                 }
//             }

//             mov(s[hi], r, DIM);
//             f[hi] = fr;
//             continue;
//         }

//         // contraction
//         point c;
//         for (int i = 0; i < DIM; i++)
//             c[i] = o[i] - contract*(o[i] - s[hi][i]);

//         float fc = objectiveFn(c);
//         if (fc < f[hi])
//         {
//             mov(s[hi], c, DIM);
//             f[hi] = fc;
//             continue;
//         }

//         // reduction
//         for (int k = 0; k < NB_POINTS; k++)
//         {
//             if (k == lo) continue;
//             for (int i = 0; i < DIM; i++)
//                 s[k][i] = s[lo][i] + shrink*(s[k][i] - s[lo][i]);
//             f[k] = objectiveFn(s[k]);
//         }
//     }

//     // return best point and its value
//     mov(pmin, s[lo], DIM);
//     return f[lo];
// }
