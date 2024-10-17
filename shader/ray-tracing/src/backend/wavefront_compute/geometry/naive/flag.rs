use std::ops::BitXor;

use crate::backend::wavefront_compute::geometry::naive::*;

#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum TraverseFlags {
  // first bits are identical to ray flag
  NONE = 0x00,
  FORCE_OPAQUE = 0x01,
  FORCE_NON_OPAQUE = 0x02,
  ACCEPT_FIRST_HIT_AND_END_SEARCH = 0x04,
  SKIP_CLOSEST_HIT_SHADER = 0x08,
  CULL_BACK_FACING_TRIANGLES = 0x10,
  CULL_FRONT_FACING_TRIANGLES = 0x20,
  CULL_OPAQUE = 0x40,
  CULL_NON_OPAQUE = 0x80,
  SKIP_TRIANGLES = 0x100,
  SKIP_PROCEDURAL_PRIMITIVES = 0x200,

  // GEOMETRY_NO_DUPLICATE_ANYHIT_INVOCATION,
  TRIANGLE_FLIP_FACING = 0x400,

  // result
  IS_OPAQUE = 0x800,
}

fn if_bit(
  source: Node<u32>,
  bit: u32,
  flag: LocalVarNode<u32>,
  if_true: impl FnOnce(Node<u32>) -> Node<u32>,
) {
  if_by((source & val(bit)).greater_than(val(0)), || {
    flag.store(if_true(flag.load()))
  });
}

impl TraverseFlags {
  pub fn from_ray_flag(ray_flag: Node<u32>) -> LocalVarNode<u32> {
    ray_flag.make_local_var()
  }

  pub fn apply_geometry_instance_flag(
    traverse_flag: LocalVarNode<u32>,
    geometry_instance_flag: Node<u32>,
  ) {
    use TraverseFlags::*;

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_TRIANGLE_FACING_CULL_DISABLE,
      traverse_flag,
      |flag| flag & val(!(CULL_BACK_FACING_TRIANGLES as u32 | CULL_FRONT_FACING_TRIANGLES as u32)),
    );

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_TRIANGLE_FLIP_FACING,
      traverse_flag,
      |flag| flag ^ val(TRIANGLE_FLIP_FACING as u32),
    );

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_FORCE_OPAQUE,
      traverse_flag,
      |flag| flag | val(FORCE_OPAQUE as u32),
    );
    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_FORCE_NO_OPAQUE,
      traverse_flag,
      |flag| flag | val(FORCE_NON_OPAQUE as u32),
    );
  }

  pub fn apply_geometry_flag_and_cull(
    traverse_flag: LocalVarNode<u32>,
    geometry_flag: Node<u32>,
  ) -> Node<bool> {
    use TraverseFlags::*;

    let flag = traverse_flag.load();
    let geometry_opaque = (geometry_flag & val(GEOMETRY_FLAG_OPAQUE)).greater_than(val(0));
    let force_opaque = (flag & val(FORCE_OPAQUE as u32)).greater_than(val(0));
    let force_non_opaque = (flag & val(FORCE_NON_OPAQUE as u32)).greater_than(val(0));
    let cull_opaque = (flag & val(CULL_OPAQUE as u32)).greater_than(0);
    let cull_non_opaque = (flag & val(CULL_NON_OPAQUE as u32)).greater_than(0);

    // write IS_OPAQUE
    let is_opaque = geometry_opaque.or(force_opaque).and(force_non_opaque.not());
    traverse_flag.store(
      traverse_flag.load() & val(!(IS_OPAQUE as u32))
        | (is_opaque.into_u32() * val(IS_OPAQUE as u32)),
    );

    is_opaque
      .and(cull_opaque)
      .or(is_opaque.not().and(cull_non_opaque))
  }

  pub fn cull_triangle(
    traverse_flag: LocalVarNode<u32>,
    is_ccw_in_local: Node<bool>,
  ) -> Node<bool> {
    use TraverseFlags::*;
    let flag = traverse_flag.load();
    let flip = (flag & val(TRIANGLE_FLIP_FACING as u32)).greater_than(val(0));
    let cull_front = (flag & val(CULL_FRONT_FACING_TRIANGLES as u32)).greater_than(val(0));
    let cull_back = (flag & val(CULL_BACK_FACING_TRIANGLES as u32)).greater_than(val(0));

    let is_front = is_ccw_in_local.bitxor(flip);
    is_front.and(cull_front).or(is_front.not().and(cull_back))
  }
}
