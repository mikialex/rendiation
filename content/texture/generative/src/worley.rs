use rendiation_algebra::*;

pub struct WorleyNoise {
  repeat: i32,
  hasher: Hasher,
  _offset_0: i32,
  _offset_1: i32,
}

impl TextureGenerator for WorleyNoise {
  type Pixel = f32;
  fn gen(&self, p: Vec2<usize>) -> Self::Pixel {
    let (x, y) = p.map(|v| v as f32).into();
    self.get((x, y, 0.).into())
  }
}

impl WorleyNoise {
  pub fn new(repeat: i32) -> Self {
    Self {
      repeat,
      hasher: Hasher::new(),
      _offset_0: 100000,
      _offset_1: 200000,
    }
  }

  fn repeat(&self, mut i: i32) -> i32 {
    i %= self.repeat;
    if i < 0 {
      i += self.repeat;
    }
    i
  }

  fn get_cell_id(&self, cell: Vec3<i32>) -> i32 {
    (self.repeat(cell.z) * self.repeat + self.repeat(cell.y)) * self.repeat + self.repeat(cell.x)
  }

  fn get_cell_feature_point(&self, cell: Vec3<i32>) -> Vec3<f32> {
    let id = self.get_cell_id(cell);
    Vec3::new(
      self.hasher.hash_f(id) + cell.x as f32,
      self.hasher.hash_f(id + self._offset_0) + cell.y as f32,
      self.hasher.hash_f(id + self._offset_1) + cell.z as f32,
    )
  }

  fn distance_to_feature(&self, point: Vec3<f32>, cell: Vec3<i32>) -> f32 {
    let feature_point = self.get_cell_feature_point(cell);
    feature_point.distance(point)
  }

  pub fn get(&self, point: Vec3<f32>) -> f32 {
    // for any given point3d.min(ceil to get a cell position;
    let point = point / self.repeat as f32;
    let cx = point.x.floor() as i32;
    let cy = point.y.floor() as i32;
    let cz = point.z.floor() as i32;

    let mut d = self.distance_to_feature(point, Vec3::new(cx, cy, cz));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy - 1, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy - 1, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy - 1, cz - 1)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy, cz - 1)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy + 1, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy + 1, cz - 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy + 1, cz - 1)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy - 1, cz)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy - 1, cz)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy - 1, cz)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy, cz)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy, cz)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy + 1, cz)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy + 1, cz)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy + 1, cz)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy - 1, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy - 1, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy - 1, cz + 1)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy, cz + 1)));

    d = d.min(self.distance_to_feature(point, Vec3::new(cx - 1, cy + 1, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx, cy + 1, cz + 1)));
    d = d.min(self.distance_to_feature(point, Vec3::new(cx + 1, cy + 1, cz + 1)));

    d / 3f32.sqrt()
  }
}

use std::num::Wrapping;

use crate::TextureGenerator;

// const PRIME32_1: Wrapping<u32> = Wrapping(2654435761);
const PRIME32_1: Wrapping<u32> = Wrapping(2246822519);
const PRIME32_2: Wrapping<u32> = Wrapping(3266489917);
const PRIME32_3: Wrapping<u32> = Wrapping(668265263);
const PRIME32_4: Wrapping<u32> = Wrapping(374761393);

struct Hasher {
  seeds: Wrapping<u32>,
}

impl Hasher {
  pub fn new() -> Self {
    Hasher { seeds: Wrapping(0) }
  }

  fn rotl32(x: u32, r: u32) -> Wrapping<u32> {
    Wrapping((x << r) | (x >> (32 - r)))
  }

  pub fn hash(&self, value: i32) -> u32 {
    let mut h32 = self.seeds + PRIME32_4;
    h32 += Wrapping(4);
    h32 += Wrapping(value as u32) * PRIME32_2;
    h32 = Hasher::rotl32(h32.0, 17) * PRIME32_3;
    h32 ^= h32 >> 15;
    h32 *= PRIME32_1;
    h32 ^= h32 >> 13;
    h32 *= PRIME32_2;
    h32 ^= h32 >> 16;
    h32.0
  }

  pub fn hash_f(&self, value: i32) -> f32 {
    self.hash(value) as f32 / std::u32::MAX as f32
  }
}
