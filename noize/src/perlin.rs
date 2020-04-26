use rendiation_math::vec::Lerp;
use rendiation_math::{One, Vec3};

/// https://flafla2.github.io/2014/08/09/perlinnoise.html
pub struct PerlinNoise {
  seed: usize,
  repeat: i32,
}

impl PerlinNoise {
  pub fn new(seed: usize) -> Self {
    Self { seed, repeat: 1 }
  }

  fn get_gradient_vec(corner_point: Vec3<i32>) -> Vec3<f32> {
    todo!()
  }

  /// Fade function as defined by Ken Perlin.  
  /// This eases coordinate values so that they will ease towards integral values.  
  /// This ends up smoothing the final output.
  /// 6t^5 - 15t^4 + 10t^3
  fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6. - 15.) + 10.)
  }

  pub fn get(&self, point: Vec3<f32>) -> f32 {
    let near_corner = point.map(|v| v.floor());
    let far_corner = near_corner + Vec3::one();

    let near_corner_i = near_corner.map(|v| v as i32);
    let far_corner_i = near_corner_i + Vec3::one();

    let i000 = Vec3::new(near_corner_i.x, near_corner_i.y, near_corner_i.z);
    let i100 = Vec3::new(far_corner_i.x, near_corner_i.y, near_corner_i.z);
    let i010 = Vec3::new(near_corner_i.x, far_corner_i.y, near_corner_i.z);
    let i110 = Vec3::new(far_corner_i.x, far_corner_i.y, near_corner_i.z);
    let i001 = Vec3::new(near_corner_i.x, near_corner_i.y, far_corner_i.z);
    let i101 = Vec3::new(far_corner_i.x, near_corner_i.y, far_corner_i.z);
    let i011 = Vec3::new(near_corner_i.x, far_corner_i.y, far_corner_i.z);
    let i111 = Vec3::new(far_corner_i.x, far_corner_i.y, far_corner_i.z);

    let g000 = PerlinNoise::get_gradient_vec(i000);
    let g100 = PerlinNoise::get_gradient_vec(i100);
    let g010 = PerlinNoise::get_gradient_vec(i010);
    let g110 = PerlinNoise::get_gradient_vec(i110);
    let g001 = PerlinNoise::get_gradient_vec(i001);
    let g101 = PerlinNoise::get_gradient_vec(i101);
    let g011 = PerlinNoise::get_gradient_vec(i011);
    let g111 = PerlinNoise::get_gradient_vec(i111);

    let center = far_corner - point;

    let f000 = (Vec3::new(near_corner.x, near_corner.y, near_corner.z) - center).dot(g000);
    let f100 = (Vec3::new(far_corner.x, near_corner.y, near_corner.z) - center).dot(g100);
    let f010 = (Vec3::new(near_corner.x, far_corner.y, near_corner.z) - center).dot(g010);
    let f110 = (Vec3::new(far_corner.x, far_corner.y, near_corner.z) - center).dot(g110);
    let f001 = (Vec3::new(near_corner.x, near_corner.y, far_corner.z) - center).dot(g001);
    let f101 = (Vec3::new(far_corner.x, near_corner.y, far_corner.z) - center).dot(g101);
    let f011 = (Vec3::new(near_corner.x, far_corner.y, far_corner.z) - center).dot(g011);
    let f111 = (Vec3::new(far_corner.x, far_corner.y, far_corner.z) - center).dot(g111);

    let u = PerlinNoise::fade(center.x);
    let v = PerlinNoise::fade(center.y);
    let w = PerlinNoise::fade(center.z);

    let x1 = f000.lerp(f100, u);
    let x2 = f010.lerp(f110, u);

    let y1 = x1.lerp(x2, v);

    let x1 = f001.lerp(f101, u);
    let x2 = f011.lerp(f111, u);

    let y2 = x1.lerp(x2, v);

    let value = y1.lerp(y2, w);

    0.5 * (value + 1.0)
  }
}
