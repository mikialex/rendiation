use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_lighting_core::*;
use rendiation_statistics::RngSampler;

pub trait Medium {
  fn is_emissive(&self) -> bool;

  /// returns information about the scattering and emission properties of the medium at a specified
  /// rendering-space point in the form of a MediumProperties object.
  fn sample_point(&self, position: Vec3<f32>) -> MediumProperties;
  /// provides information about the medium’s majorant sigma_majorant along the ray’s max_distance
  /// extent.
  fn sample_ray(
    &self,
    ray: Ray3,
    max_distance: f32,
    lambda: &SampledWaveLengths,
  ) -> Box<MajorantIterator>; // todo, remove allocation
}

pub struct MediumProperties {
  pub sigma_a: SampledSpectrum,
  pub sigma_s: SampledSpectrum,
  pub phase: PhaseFunction,
  pub le: SampledSpectrum,
}

pub struct RayMajorantSegment {
  pub min: f32,
  pub max: f32,
  pub value: SampledSpectrum,
}

impl RayMajorantSegment {
  pub fn length(&self) -> f32 {
    self.max - self.min
  }
}

pub type MajorantIterator = dyn Iterator<Item = RayMajorantSegment>;

pub fn sample_transmittance_majorants(
  medium: &dyn Medium,
  ray: Ray3,
  rng: &RngSampler,
  max_distance: f32,
  lambda: &SampledWaveLengths,
) -> SampledSpectrum {
  let mut transmittance_majorants = SampledSpectrum::new_fill_with(1.0);
  for majorant_segment in &mut medium.sample_ray(ray, max_distance, lambda) {
    // Handle zero-valued majorant for current segment
    if majorant_segment.value.is_all_zero() {
      // transmittance_majorants *= ((-majorant_segment.length() * majorant_segment.value).exp());
      continue;
    }

    // // Generate samples along current majorant segment
    // let mut t_min = majorant_segment.min;
    // loop {
    //   // Try to generate sample along current majorant segment
    //   let t = t_min + SampleExponential(rng.sample(), majorant_segment.value);
    //   // u = rng.Uniform<Float>();
    //   if t < seg.max {
    //     // // Call callback function for sample within segment
    //     // T_maj *= FastExp(-(t - tMin) * seg->sigma_maj);
    //     // MediumProperties mp = medium->SamplePoint(ray(t), lambda);
    //     // if (!callback(ray(t), mp, seg->sigma_maj, T_maj)) {
    //     //     // Returning out of doubly-nested while loop is not as good perf. wise
    //     //     // on the GPU vs using "done" here.
    //     //     done = true;
    //     //     break;
    //     // }
    //     // T_maj = SampledSpectrum(1.f);
    //     // tMin = t;
    //   } else {
    //     // Handle sample past end of majorant segment
    //     // transmittance_majorants *= ((-majorant_segment.length() *
    // majorant_segment.value).exp());     break;
    //   }
    // }
  }

  transmittance_majorants
}

// todo
type PhaseFunction = f32;

// todo
type Spectrum = Vec3<f32>;

pub struct HomogeneousMedium {
  /// absorption coefficient
  ///
  ///  the probability density that light is absorbed per unit distance traveled in the medium
  pub sigma_a: Spectrum,

  /// scattering coefficient
  ///
  /// The probability of an out-scattering event occurring per unit distance
  pub sigma_s: Spectrum,
  pub g: f32,
}

impl HomogeneousMedium {
  /// The total reduction in radiance due to absorption and out scattering is given by the sum .
  /// This combined effect of absorption and out scattering is called attenuation or extinction.
  pub fn sigma_t(&self) -> Spectrum {
    self.sigma_a + self.sigma_s
  }
}

pub struct HenyeyGreenstein {
  pub g: f32,
}

impl HenyeyGreenstein {
  pub fn p(&self, wo: &Vec3<f32>, wi: &Vec3<f32>) -> f32 {
    Self::phase_hg(wo.dot(*wi), self.g)
  }
  pub fn sample_p(&self, wo: &Vec3<f32>, wi: &mut Vec3<f32>, u: Vec2<f32>) -> f32 {
    // compute $\cos \theta$ for HenyeyGreenstein sample
    let cos_theta = if self.g.abs() < 1e-3 {
      1.0 - 2.0 * u.x
    } else {
      let sqr_term = (1.0 - self.g * self.g) / (1.0 + self.g - 2.0 * self.g * u.x);

      -(1.0 + self.g * self.g - sqr_term * sqr_term) / (2.0 * self.g)
    };
    // compute direction _wi_ for HenyeyGreenstein sample
    let sin_theta = 0.0.max(1.0 - cos_theta * cos_theta).sqrt();
    let phi = 2.0 * f32::PI() * u.y;
    let mut v1: Vec3<f32> = Vec3::default();
    let mut v2: Vec3<f32> = Vec3::default();
    vec3_coordinate_system(wo, &mut v1, &mut v2);
    *wi = spherical_direction_vec3(sin_theta, cos_theta, phi, &v1, &v2, wo);
    Self::phase_hg(cos_theta, self.g)
  }

  pub fn phase_hg(cos_theta: f32, g: f32) -> f32 {
    let denom = 1.0 + g * g + 2.0 * g * cos_theta;
    (1.0 / (4.0 * f32::PI())) * (1.0 - g * g) / (denom * denom.sqrt())
  }
}

/// Construct a local coordinate system given only a single 3D vector.
pub fn vec3_coordinate_system(v1: &Vec3<f32>, v2: &mut Vec3<f32>, v3: &mut Vec3<f32>) {
  if v1.x.abs() > v1.y.abs() {
    *v2 = Vec3 {
      x: -v1.z,
      y: 0.0,
      z: v1.x,
    } / (v1.x * v1.x + v1.z * v1.z).sqrt();
  } else {
    *v2 = Vec3 {
      x: 0.0,
      y: v1.z,
      z: -v1.y,
    } / (v1.y * v1.y + v1.z * v1.z).sqrt();
  }
  *v3 = v1.cross(*v2);
}

/// Take three basis vectors representing the x, y, and z axes and
/// return the appropriate direction vector with respect to the
/// coordinate frame defined by them.
pub fn spherical_direction_vec3(
  sin_theta: f32,
  cos_theta: f32,
  phi: f32,
  x: &Vec3<f32>,
  y: &Vec3<f32>,
  z: &Vec3<f32>,
) -> Vec3<f32> {
  *x * (sin_theta * phi.cos()) + *y * (sin_theta * phi.sin()) + *z * cos_theta
}
