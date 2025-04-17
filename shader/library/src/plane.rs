use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

impl ShaderPlane {
  pub fn new(normal: Vec3<f32>, constant: f32) -> Self {
    Self {
      normal,
      constant,
      ..Zeroable::zeroed()
    }
  }
}

pub fn ray_plane_intersect(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  plane: ENode<ShaderPlane>,
) -> Node<Vec4<f32>> {
  let denominator = plane.normal.dot(direction); // I don't care if it's zero!

  let t = -(plane.normal.dot(origin) + plane.constant) / denominator;

  t.greater_equal_than(0.)
    .select((origin + direction * t, val(1.0)), Vec4::zero())
}

pub fn ray_aabb_intersect(
  min: Node<Vec3<f32>>,
  max: Node<Vec3<f32>>,
  plane: ENode<ShaderPlane>,
) -> Node<bool> {
  let normal = plane.normal;
  let x = normal.x().greater_than(0.).select(max.x(), min.x());
  let y = normal.y().greater_than(0.).select(max.y(), min.y());
  let z = normal.z().greater_than(0.).select(max.z(), min.z());
  let point: Node<Vec3<_>> = (x, y, z).into();
  let distance = normal.dot(point) + plane.constant;
  distance.less_than(0.)
}
