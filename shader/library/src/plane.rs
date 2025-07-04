use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPlaneUniform {
  pub normal: Vec3<f32>,
  // todo, consider using a single HighPrecisionFloat
  pub position: HighPrecisionTranslationUniform,
}

impl ShaderPlaneUniform {
  pub fn new(normal: Vec3<f32>, constant: f64) -> Self {
    let position = normal.into_f64() * constant;
    let position = into_hpt(position).into_uniform();
    Self {
      normal,
      position,
      ..Zeroable::zeroed()
    }
  }

  pub fn into_shader_plane(
    plane: Node<Self>,
    camera_position: Node<HighPrecisionTranslation>,
  ) -> ENode<ShaderPlane> {
    let plane = plane.expand();
    let position = hpt_sub_hpt(hpt_uniform_to_hpt(plane.position), camera_position);
    ENode::<ShaderPlane> {
      normal: plane.normal,
      constant: position.length(),
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
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

pub fn aabb_plane_intersect(
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
