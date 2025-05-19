use crate::*;

#[derive(Copy, Clone, ShaderStruct)]
pub struct CubeFaceInfo {
  pub face_id: i32,
  pub uv: Vec2<f32>,
}

#[shader_fn]
pub fn screen_position_to_cube_face(position: Node<Vec2<f32>>) -> Node<CubeFaceInfo> {
  let face_id = val(-1_i32).make_local_var();
  let uv = val(Vec2::<f32>::zero()).make_local_var();

  let x = position.x();
  let y = position.y();

  if_by(y.less_than(1. / 3.), || {
    if_by(x.greater_than(0.25).and(x.less_than(0.5)), || {
      uv.store(((x - val(0.25)) * val(4.), y * val(3.)));
      face_id.store(2);
    });
  })
  .else_if(y.greater_than(2. / 3.), || {
    if_by(x.greater_than(0.25).and(x.less_than(0.5)), || {
      uv.store(((x - val(0.25)) * val(4.), (y - val(2. / 3.)) * val(3.)));
      face_id.store(3);
    });
  })
  .else_by(|| {
    if_by(x.less_than(0.25), || {
      uv.store((x * val(4.), (y - val(1. / 3.)) * val(3.)));
      face_id.store(1);
    })
    .else_if(x.greater_than(0.25).and(x.less_than(0.5)), || {
      uv.store(((x - val(0.25)) * val(4.), (y - val(1. / 3.)) * val(3.)));
      face_id.store(4);
    })
    .else_if(x.greater_than(0.5).and(x.less_than(0.75)), || {
      uv.store(((x - val(0.5)) * val(4.), (y - val(1. / 3.)) * val(3.)));
      face_id.store(0);
    })
    .else_by(|| {
      uv.store(((x - val(0.75)) * val(4.), (y - val(1. / 3.)) * val(3.)));
      face_id.store(5);
    });
  });

  ENode::<CubeFaceInfo> {
    face_id: face_id.load(),
    uv: uv.load(),
  }
  .construct()
}

#[shader_fn]
pub fn direction_for(face: Node<i32>, uv: Node<Vec2<f32>>) -> Node<Vec3<f32>> {
  let result = val(Vec3::<f32>::zero()).make_local_var();
  let uv = val(2.) * uv - val(1.).splat();

  switch_by(face)
    .case(0, || {
      result.store((val(1.0), -uv.y(), -uv.x()));
    })
    .case(1, || {
      result.store((val(-1.0), -uv.y(), uv.x()));
    })
    .case(2, || {
      result.store((uv.x(), val(1.0), uv.y()));
    })
    .case(3, || {
      result.store((uv.x(), val(-1.0), -uv.y()));
    })
    .case(4, || {
      result.store((uv.x(), -uv.y(), val(1.0)));
    })
    .case(5, || {
      result.store((-uv.x(), -uv.y(), val(-1.0)));
    })
    .end_with_default(|| {});

  result.load()
}

#[shader_fn]
pub fn get_cube_face_index_by_dir(dir: Node<Vec3<f32>>) -> Node<i32> {
  let tolerance = 0.0001;
  let abs_x = dir.x().abs();
  let abs_y = dir.y().abs();
  let abs_z = dir.z().abs();
  let max = abs_x.max(abs_y).max(abs_z);

  let index = val(0_i32).make_local_var();

  if_by((max - abs_x).abs().less_than(tolerance), || {
    index.store(0);
    if_by(dir.x().less_than(0.), || index.store(1));
  })
  .else_if((max - abs_y).abs().less_than(tolerance), || {
    index.store(2);
    if_by(dir.y().less_than(0.), || index.store(3));
  })
  .else_if((max - abs_z).abs().less_than(tolerance), || {
    index.store(4);
    if_by(dir.z().less_than(0.), || index.store(5));
  })
  .else_over();

  index.load()
}
