use crate::*;

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
  todo!()
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

// // computes
// // * the norm (albedo) of the BRDF
// // * the average Schlick Fresnel value
// // * the average direction of the BRDF
// void computeAvgTerms(const Brdf& brdf, const vec3& V, const float alpha,
//     float& norm, float& fresnel, vec3& averageDir)
// {
//     norm = 0.0f;
//     fresnel = 0.0f;
//     averageDir = vec3(0, 0, 0);

//     for (int j = 0; j < Nsample; ++j)
//     for (int i = 0; i < Nsample; ++i)
//     {
//         const float U1 = (i + 0.5f)/Nsample;
//         const float U2 = (j + 0.5f)/Nsample;

//         // sample
//         const vec3 L = brdf.sample(V, alpha, U1, U2);

//         // eval
//         float pdf;
//         float eval = brdf.eval(V, L, alpha, pdf);

//         if (pdf > 0)
//         {
//             float weight = eval / pdf;

//             vec3 H = normalize(V+L);

//             // accumulate
//             norm       += weight;
//             fresnel    += weight * pow(1.0f - glm::max(dot(V, H), 0.0f), 5.0f);
//             averageDir += weight * L;
//         }
//     }

//     norm    /= (float)(Nsample*Nsample);
//     fresnel /= (float)(Nsample*Nsample);

//     // clear y component, which should be zero with isotropic BRDFs
//     averageDir.y = 0.0f;

//     averageDir = normalize(averageDir);
// }

// // compute the error between the BRDF and the LTC
// // using Multiple Importance Sampling
// float computeError(const LTC& ltc, const Brdf& brdf, const vec3& V, const float alpha)
// {
//     double error = 0.0;

//     for (int j = 0; j < Nsample; ++j)
//     for (int i = 0; i < Nsample; ++i)
//     {
//         const float U1 = (i + 0.5f)/Nsample;
//         const float U2 = (j + 0.5f)/Nsample;

//         // importance sample LTC
//         {
//             // sample
//             const vec3 L = ltc.sample(U1, U2);

//             float pdf_brdf;
//             float eval_brdf = brdf.eval(V, L, alpha, pdf_brdf);
//             float eval_ltc = ltc.eval(L);
//             float pdf_ltc = eval_ltc/ltc.magnitude;

//             // error with MIS weight
//             double error_ = fabsf(eval_brdf - eval_ltc);
//             error_ = error_*error_*error_;
//             error += error_/(pdf_ltc + pdf_brdf);
//         }

//         // importance sample BRDF
//         {
//             // sample
//             const vec3 L = brdf.sample(V, alpha, U1, U2);

//             float pdf_brdf;
//             float eval_brdf = brdf.eval(V, L, alpha, pdf_brdf);
//             float eval_ltc = ltc.eval(L);
//             float pdf_ltc = eval_ltc/ltc.magnitude;

//             // error with MIS weight
//             double error_ = fabsf(eval_brdf - eval_ltc);
//             error_ = error_*error_*error_;
//             error += error_/(pdf_ltc + pdf_brdf);
//         }
//     }

//     return (float)error / (float)(Nsample*Nsample);
// }

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

// float sqr(float x)
// {
//     return x*x;
// }

// float G(float w, float s, float g)
// {
//     return -2.0f*sinf(w)*cosf(s)*cosf(g) + pi/2.0f - g + sinf(g)*cosf(g);
// }

// float H(float w, float s, float g)
// {
//     float sinsSq = sqr(sin(s));
//     float cosgSq = sqr(cos(g));

//     return cosf(w)*(cosf(g)*sqrtf(sinsSq - cosgSq) + sinsSq*asinf(cosf(g)/sinf(s)));
// }

// float ihemi(float w, float s)
// {
//     float g = asinf(cosf(s)/sinf(w));
//     float sinsSq = sqr(sinf(s));

//     if (w >= 0.0f && w <= (pi/2.0f - s))
//         return pi*cosf(w)*sinsSq;

//     if (w >= (pi/2.0f - s) && w < pi/2.0f)
//         return pi*cosf(w)*sinsSq + G(w, s, g) - H(w, s, g);

//     if (w >= pi/2.0f && w < (pi/2.0f + s))
//         return G(w, s, g) + H(w, s, g);

//     return 0.0f;
// }

// void genSphereTab(float* tabSphere, int N)
// {
//     for (int j = 0; j < N; ++j)
//     for (int i = 0; i < N; ++i)
//     {
//         const float U1 = float(i)/(N - 1);
//         const float U2 = float(j)/(N - 1);

//         // z = cos(elevation angle)
//         float z = 2.0f*U1 - 1.0f;

//         // length of average dir., proportional to sin(sigma)^2
//         float len = U2;

//         float sigma = asinf(sqrtf(len));
//         float omega = acosf(z);

//         // compute projected (cosine-weighted) solid angle of spherical cap
//         float value = 0.0f;

//         if (sigma > 0.0f)
//             value = ihemi(omega, sigma)/(pi*len);
//         else
//             value = std::max<float>(z, 0.0f);

//         if (value != value)
//             printf("nan!\n");

//         tabSphere[i + j*N] = value;
//     }
// }
