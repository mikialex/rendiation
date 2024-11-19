use crate::*;

/// Avoiding self intersections (see Ray Tracing Gems, Ch. 6)
/// return the position after offset
#[shader_fn]
pub fn offset_ray_hit(position: Node<Vec3<f32>>, normal: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let int_scale = val(256.0);
  let float_scale = val(1.0 / 65536.0);
  let origin = val(1.0 / 32.0);

  let of_i = normal * int_scale.splat::<Vec3<_>>();
  let of_i: Node<Vec3<i32>> = (
    of_i.x().into_i32(),
    of_i.y().into_i32(),
    of_i.z().into_i32(),
  )
    .into();

  let p_i_x = position.x().less_than(0.).select(-of_i.x(), of_i.x());
  let p_i_x = (position.x().bitcast::<i32>() + p_i_x).bitcast::<f32>();

  let p_i_y = position.y().less_than(0.).select(-of_i.y(), of_i.y());
  let p_i_y = (position.y().bitcast::<i32>() + p_i_y).bitcast::<f32>();

  let p_i_z = position.z().less_than(0.).select(-of_i.z(), of_i.z());
  let p_i_z = (position.z().bitcast::<i32>() + p_i_z).bitcast::<f32>();

  let r_x = position
    .x()
    .abs()
    .less_than(origin)
    .select(position.x() + float_scale * normal.x(), p_i_x);
  let r_y = position
    .y()
    .abs()
    .less_than(origin)
    .select(position.y() + float_scale * normal.y(), p_i_y);
  let r_z = position
    .z()
    .abs()
    .less_than(origin)
    .select(position.z() + float_scale * normal.z(), p_i_z);

  (r_x, r_y, r_z).into()
}
