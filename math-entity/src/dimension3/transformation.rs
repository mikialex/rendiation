use rendiation_math::*;

#[derive(Default)]
pub struct Transformation {
  pub matrix: Mat4<f32>,
  pub position: Vec3<f32>,
  pub scale: Vec3<f32>,
  pub rotation: Quat<f32>,
}

impl Transformation {
  pub fn new() -> Self {
    Self {
      position: Vec3::new(0.0, 0.0, 0.0),
      scale: Vec3::new(1.0, 1.0, 1.0),
      rotation: Quat::new(0.0, 0.0, 0.0, 1.0),
      matrix: Mat4::one(),
    }
  }

  pub fn update_matrix_by_compose(&mut self) {
    self.matrix = compose(&self.position, &self.rotation, &self.scale);
  }
}

// one step fast compose
pub fn compose(position: &Vec3<f32>, quaternion: &Quat<f32>, scale: &Vec3<f32>) -> Mat4<f32> {
  let x = quaternion.x;
  let y = quaternion.y;
  let z = quaternion.z;
  let w = quaternion.w;
  let x2 = x + x;
  let y2 = y + y;
  let z2 = z + z;
  let xx = x * x2;
  let xy = x * y2;
  let xz = x * z2;
  let yy = y * y2;
  let yz = y * z2;
  let zz = z * z2;
  let wx = w * x2;
  let wy = w * y2;
  let wz = w * z2;

  let sx = scale.x;
  let sy = scale.y;
  let sz = scale.z;

  Mat4::new(
    (1. - (yy + zz)) * sx,
    (xy + wz) * sx,
    (xz - wy) * sx,
    0.,
    (xy - wz) * sy,
    (1. - (xx + zz)) * sy,
    (yz + wx) * sy,
    0.,
    (xz + wy) * sz,
    (yz - wx) * sz,
    (1. - (xx + yy)) * sz,
    0.,
    position.x,
    position.y,
    position.z,
    1.,
  )
}

// pub fn decompose ( position: &Vec3<f32>, quaternion: &Quat<f32>, scale: &Vec3<f32> ) {

//   var te = this.elements;

//   var sx = _v1.set( te[ 0 ], te[ 1 ], te[ 2 ] ).length();
//   var sy = _v1.set( te[ 4 ], te[ 5 ], te[ 6 ] ).length();
//   var sz = _v1.set( te[ 8 ], te[ 9 ], te[ 10 ] ).length();

//   // if determine is negative, we need to invert one scale
//   var det = this.determinant();
//   if ( det < 0 ) sx = - sx;

//   position.x = te[ 12 ];
//   position.y = te[ 13 ];
//   position.z = te[ 14 ];

//   // scale the rotation part
//   _m1.copy( this );

//   var invSX = 1 / sx;
//   var invSY = 1 / sy;
//   var invSZ = 1 / sz;

//   _m1.elements[ 0 ] *= invSX;
//   _m1.elements[ 1 ] *= invSX;
//   _m1.elements[ 2 ] *= invSX;

//   _m1.elements[ 4 ] *= invSY;
//   _m1.elements[ 5 ] *= invSY;
//   _m1.elements[ 6 ] *= invSY;

//   _m1.elements[ 8 ] *= invSZ;
//   _m1.elements[ 9 ] *= invSZ;
//   _m1.elements[ 10 ] *= invSZ;

//   quaternion.setFromRotationMatrix( _m1 );

//   scale.x = sx;
//   scale.y = sy;
//   scale.z = sz;

//   return this;

// }
