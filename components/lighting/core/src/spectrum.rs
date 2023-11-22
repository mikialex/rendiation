#![allow(clippy::excessive_precision)]

pub fn blackbody(lambda: &[f32], n: usize, t: f32, le: &mut Vec<f32>) {
  if t <= 0.0 {
    for _i in 0..n {
      le.push(0.0);
    }
    return;
  }
  let c: f32 = 299_792_458.0;
  let h: f32 = 6.626_069_57e-34;
  let kb: f32 = 1.380_648_8e-23;
  for item in lambda.iter().take(n) {
    let lambda_i: f32 = *item;
    let l: f32 = lambda_i * 1.0e-9;
    let lambda5: f32 = (l * l) * (l * l) * l;
    let e: f32 = ((h * c) / (l * kb * t)).exp();
    let lei: f32 = (2.0 * h * c * c) / (lambda5 * (e - 1.0));
    assert!(!lei.is_nan());
    le.push(lei);
  }
}

pub fn blackbody_normalized(lambda: &[f32], n: usize, t: f32, le: &mut Vec<f32>) {
  blackbody(lambda, n, t, le);
  // normalize _Le_ values based on maximum blackbody radiance
  let lambda_max: [f32; 1] = [2.897_772_1e-3 / t * 1.0e9];
  let mut max_l: Vec<f32> = Vec::new();
  blackbody(&lambda_max, 1, t, &mut max_l);
  for item in le.iter_mut().take(n) {
    *item /= max_l[0];
  }
}

/// represents a constant spectral distribution over all wavelengths
pub struct ConstantSpectrum {
  pub c: f32,
}

/// a spectral distribution sampled at 1 nm intervals over a given range of integer wavelengths
pub struct DenselySampledSpectrum {
  pub values: Vec<f32>,
  pub lambda_min: u32,
  pub lambda_max: u32,
}

/// the spectral distribution of a blackbody emitter at a specified temperature.
pub struct BlackbodySpectrum {
  /// in kelvin
  pub temperature: f32,
}

pub const SPECTRUM_SAMPLE_COUNT: usize = 4;
/// represent values of the spectral distribution at discrete wavelengths
///
/// I should use other math lib to do this work
#[derive(Debug, Copy, Clone)]
pub struct SampledSpectrum {
  pub samples: [f32; SPECTRUM_SAMPLE_COUNT],
}

impl std::ops::Mul<f32> for SampledSpectrum {
  type Output = Self;

  fn mul(mut self, rhs: f32) -> Self::Output {
    self.samples.iter_mut().for_each(|v| *v *= rhs);
    self
  }
}

impl std::ops::Mul<Self> for SampledSpectrum {
  type Output = Self;

  fn mul(mut self, rhs: Self) -> Self::Output {
    self
      .samples
      .iter_mut()
      .zip(rhs.samples.iter())
      .for_each(|(v, rhs)| *v *= rhs);
    self
  }
}
impl std::ops::MulAssign for SampledSpectrum {
  fn mul_assign(&mut self, rhs: Self) {
    *self = *self * rhs
  }
}

impl std::ops::Neg for SampledSpectrum {
  type Output = Self;

  fn neg(self) -> Self::Output {
    self * (-1.)
  }
}

impl SampledSpectrum {
  pub fn new_fill_with(init: f32) -> Self {
    Self {
      samples: [init; SPECTRUM_SAMPLE_COUNT],
    }
  }

  pub fn exp(mut self) -> Self {
    self.samples.iter_mut().for_each(|v| *v = v.exp());
    self
  }

  /// It is often useful to know if all the values in a SampledSpectrum are zero. For example, if a
  /// surface has zero reflectance, then the light transport routines can avoid the computational
  /// cost of casting reflection rays that have contributions that would eventually be multiplied by
  /// zeros.
  pub fn is_all_zero(&self) -> bool {
    for sample in self.samples {
      if sample != 0. {
        return false;
      }
    }
    true
  }
}

/// https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Representing_Spectral_Distributions#NSpectrumSamples
///
/// SampledWavelengths, stores the wavelengths for which a SampledSpectrum stores samples. Thus, it
/// is important not only to keep careful track of the SampledWavelengths that are represented by an
/// individual SampledSpectrum but also to not perform any operations that combine SampledSpectrums
/// that have samples at different wavelengths.
///
/// Now that SampledWavelengths and SampledSpectrum have been introduced, it is reasonable to ask
/// the question: why are they separate classes, rather than a single class that stores both
/// wavelengths and their sample values? Indeed, an advantage of such a design would be that it
/// would be possible to detect at runtime if an operation was performed with two SampledSpectrum
/// instances that stored values for different wavelengths—such an operation is nonsensical and
/// would signify a bug in the system.
///
/// However, in practice many SampledSpectrum objects are created during rendering, many as
/// temporary values in the course of evaluating expressions involving spectral computation. It is
/// therefore worthwhile to minimize the object’s size, if only to avoid initialization and copying
/// of additional data.
pub struct SampledWaveLengths {
  pub lambda: [f32; SPECTRUM_SAMPLE_COUNT],
  pub pdf: [f32; SPECTRUM_SAMPLE_COUNT],
}
