pub fn blackbody(lambda: &[f32], n: usize, t: f32, le: &mut Vec<f32>) {
  if t <= 0.0 as f32 {
    for _i in 0..n {
      le.push(0.0 as f32);
    }
    return;
  }
  let c: f32 = 299_792_458.0 as f32;
  let h: f32 = 6.626_069_57e-34 as f32;
  let kb: f32 = 1.380_648_8e-23 as f32;
  for item in lambda.iter().take(n) {
    let lambda_i: f32 = *item;
    let l: f32 = (lambda_i as f64 * 1.0e-9 as f64) as f32;
    let lambda5: f32 = (l * l) * (l * l) * l;
    let e: f32 = ((h * c) / (l * kb * t)).exp();
    let lei: f32 = (2.0 as f32 * h * c * c) / (lambda5 * (e - 1.0 as f32));
    assert!(!lei.is_nan());
    le.push(lei);
  }
}

pub fn blackbody_normalized(lambda: &[f32], n: usize, t: f32, le: &mut Vec<f32>) {
  blackbody(lambda, n, t, le);
  // normalize _Le_ values based on maximum blackbody radiance
  let lambda_max: [f32; 1] = [2.897_772_1e-3 as f32 / t * 1.0e9 as f32];
  let mut max_l: Vec<f32> = Vec::new();
  blackbody(&lambda_max, 1, t, &mut max_l);
  for item in le.iter_mut().take(n) {
    *item /= max_l[0];
  }
}

/// represents a constant spectral distribution over all wavelengths
pub struct ConstantSpectrum {
  c: f32,
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
  temperature: f32,
}
