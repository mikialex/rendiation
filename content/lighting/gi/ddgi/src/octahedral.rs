use crate::*;

pub fn sign_not_zero(v: Node<f32>) -> Node<f32> {
  v.greater_equal_than(0.0).select(val(1.0), val(-1.0))
}

/// Computes the normalized octahedral direction that corresponds to the
/// given normalized coordinates on the [-1, 1] square.
pub fn octahedral_coordinate_to_direction(coord: Node<Vec2<f32>>) -> Node<Vec3<f32>> {
  let direction: Node<Vec3<f32>> = (
    coord.x(),
    coord.y(),
    val(1.0) - coord.x().abs() - coord.y().abs(),
  )
    .into();
  let result = direction.make_local_var();
  if_by(direction.z().less_than(0.0), || {
    let x = (val(1.0) - direction.x().abs()) * sign_not_zero(direction.x());
    let y = (val(1.0) - direction.y().abs()) * sign_not_zero(direction.y());
    let r: Node<Vec3<f32>> = (x, y, direction.z()).into();
    result.store(r)
  });

  result.load().normalize()
}

///  Computes the octant coordinates in the normalized [-1, 1] square, for the given a unit direction vector.
pub fn direction_to_octahedral_coordinate(direction: Node<Vec3<f32>>) -> Node<Vec2<f32>> {
  let l1norm = direction.x().abs() + direction.y().abs() + direction.z().abs();
  let uv = direction.xy() * (val(1.0) / l1norm);
  let result = uv.make_local_var();
  if_by(direction.z().less_than(0.0), || {
    let x = (val(1.0) - uv.x().abs()) * sign_not_zero(uv.x());
    let y = (val(1.0) - uv.y().abs()) * sign_not_zero(uv.y());
    let r: Node<Vec2<f32>> = (x, y).into();
    result.store(r)
  });
  result.load()
}
