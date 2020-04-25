use rendiation_math::Vec3;

pub struct PerlinNoise {
  seed: usize,
  repeat: i32,
}

impl PerlinNoise {
  pub fn new(seed: usize) -> Self {
    Self { seed, repeat: 1 }
  }

  pub fn get(&self, point: Vec3<f32>) -> f32 {
    
    let floored = Vec3::new(
      point.x.floor(),
      point.y.floor(),
      point.z.floor(),
    );

    let near_corner = Vec3::new(
      point.x.floor() as i32,
      point.y.floor() as i32,
      point.z.floor() as i32,
    );

    let far_corner = near_corner + Vec3::new(1, 1, 1);
    let near_distance = point - floored;
    let far_distance = near_distance - Vec3::new(1., 1., 1.);
  }
}
