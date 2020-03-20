use rendiation_math::Vec3;

struct WorleyNoise {
  repeat: i32,
  hasher: Hasher,
  kIDOffs1: i32,
  kIDOffs2: i32,
}

impl WorleyNoise {
  pub fn new(repeat: i32) -> Self {
    Self {
      repeat,
      hasher: Hasher::new(),
      kIDOffs1: 100000,
      kIDOffs2: 200000,
    }
  }

  fn repeat(&self, mut i: i32) -> i32 {
    i %= self.repeat;
    if i < 0 {
      i += self.repeat;
    }
    i
  }

  fn get_ceil_id(&self, cell: Vec3<i32>) -> i32 {
    (self.repeat(cell.z) * self.repeat + self.repeat(cell.y)) * self.repeat + self.repeat(cell.x)
  }

  fn get_cell_feature_point(&self, cell: Vec3<i32>) -> Vec3<f32> {
    let id = self.get_ceil_id(cell);
    Vec3::new(
      self.hasher.hash_f(id) + cell.x as f32,
      self.hasher.hash_f(id + self.kIDOffs1) + cell.y as f32,
      self.hasher.hash_f(id + self.kIDOffs2) + cell.z as f32,
    )
  }

  fn distanceToFeature(&self, point: Vec3<f32>, cell: Vec3<i32>) -> f32 {
    let feature_point = self.get_cell_feature_point(cell);
    feature_point.distance(point)
  }

  pub fn get(&self, point: Vec3<f32>) -> f32 {
    // for any given point3d.min(ceil to get a cell position;
    let cell = Vec3::new(
      point.x.ceil() as i32,
      point.y.ceil() as i32,
      point.z.ceil() as i32,
    );

    let cx = point.x.ceil() as i32;
    let cy = point.y.ceil() as i32;
    let cz = point.z.ceil() as i32;

    let mut d = self.distanceToFeature(point, Vec3::new(cx, cy, cz));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy - 1, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy - 1, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy - 1, cz - 1)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy, cz - 1)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy + 1, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy + 1, cz - 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy + 1, cz - 1)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy - 1, cz)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy - 1, cz)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy - 1, cz)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy, cz)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy, cz)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy + 1, cz)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy + 1, cz)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy + 1, cz)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy - 1, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy - 1, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy - 1, cz + 1)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy, cz + 1)));

    d = d.min(self.distanceToFeature(point, Vec3::new(cx - 1, cy + 1, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx, cy + 1, cz + 1)));
    d = d.min(self.distanceToFeature(point, Vec3::new(cx + 1, cy + 1, cz + 1)));

    d
  }
}

struct Hasher {
  seeds: i32,
  PRIME32_1: u32,
  PRIME32_2: u32,
  PRIME32_3: u32,
  PRIME32_4: u32,
  PRIME32_5: u32,
}

impl Hasher {
  pub fn new() -> Self {
    Hasher {
      seeds: 0,
      PRIME32_1: 2654435761,
      PRIME32_2: 2246822519,
      PRIME32_3: 3266489917,
      PRIME32_4: 668265263,
      PRIME32_5: 374761393,
    }
  }

  fn rotl32(x: u32, r: i32) -> u32 {
    return (x << r) | (x >> 32 - r);
  }

  pub fn hash(&self, value: i32) -> u32 {
    let mut h32 = self.seeds as u32 + self.PRIME32_5;
    h32 += 4;
    h32 += value as u32 * self.PRIME32_3;
    h32 = Hasher::rotl32(h32, 17) * self.PRIME32_4;
    h32 ^= h32 >> 15;
    h32 *= self.PRIME32_2;
    h32 ^= h32 >> 13;
    h32 *= self.PRIME32_3;
    h32 ^= h32 >> 16;
    return h32;
  }

  pub fn hash_f(&self, value: i32) -> f32 {
    self.hash(value) as f32 / std::u32::MAX as f32
  }
}
