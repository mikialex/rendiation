use rand::Rng;

use crate::SampleGenerator;

pub struct SobolSamplingGenerator {
  scramble_1d: u32,
  scramble_2d: (u32, u32),
}

impl Default for SobolSamplingGenerator {
  fn default() -> Self {
    let mut rng = rand::thread_rng();
    Self {
      scramble_1d: rng.gen_range(0..u32::MAX),
      scramble_2d: (rng.gen_range(0..u32::MAX), rng.gen_range(0..u32::MAX)),
    }
  }
}

impl SampleGenerator for SobolSamplingGenerator {
  fn override_spp(&self, requested_min_spp: usize) -> usize {
    requested_min_spp.next_power_of_two()
  }

  fn gen_1d(&self, n: usize) -> f32 {
    van_der_corput(n as u32, self.scramble_1d)
  }

  fn gen_2d(&self, n: usize) -> (f32, f32) {
    (
      van_der_corput(n as u32, self.scramble_2d.0),
      sobol(n as u32, self.scramble_2d.1),
    )
  }
}

/// Generate a scrambled Van der Corput sequence value
/// as described by Kollig & Keller (2002) and in PBR
/// method is specialized for base 2
fn van_der_corput(mut n: u32, scramble: u32) -> f32 {
  n = n.rotate_right(16);
  n = ((n & 0x00ff00ff) << 8) | ((n & 0xff00ff00) >> 8);
  n = ((n & 0x0f0f0f0f) << 4) | ((n & 0xf0f0f0f0) >> 4);
  n = ((n & 0x33333333) << 2) | ((n & 0xcccccccc) >> 2);
  n = ((n & 0x55555555) << 1) | ((n & 0xaaaaaaaa) >> 1);
  n ^= scramble;
  f32::min(
    ((n >> 8) & 0xffffff) as f32 / ((1 << 24) as f32),
    1.0 - f32::EPSILON,
  )
}
/// Generate a scrambled Sobol' sequence value
/// as described by Kollig & Keller (2002) and in PBR
/// method is specialized for base 2
fn sobol(mut n: u32, mut scramble: u32) -> f32 {
  let mut i = 1 << 31;
  while n != 0 {
    if n & 0x1 != 0 {
      scramble ^= i;
    }
    n >>= 1;
    i ^= i >> 1;
  }
  f32::min(
    ((scramble >> 8) & 0xffffff) as f32 / ((1 << 24) as f32),
    1.0 - f32::EPSILON,
  )
}
