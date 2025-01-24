use crate::backend::wavefront_compute::geometry::naive::*;

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub enum TraverseFlags {
  // first bits are identical to ray flag
  _NONE = 0x00,
  FORCE_OPAQUE = 0x01,
  FORCE_NON_OPAQUE = 0x02,
  ACCEPT_FIRST_HIT_AND_END_SEARCH = 0x04,
  _SKIP_CLOSEST_HIT_SHADER = 0x08, // handled in trace_task
  CULL_BACK_FACING_TRIANGLES = 0x10,
  CULL_FRONT_FACING_TRIANGLES = 0x20,
  CULL_OPAQUE = 0x40,
  CULL_NON_OPAQUE = 0x80,
  SKIP_TRIANGLES = 0x100,
  _SKIP_BOXES = 0x200,

  // GEOMETRY_NO_DUPLICATE_ANYHIT_INVOCATION,
  TRIANGLE_FLIP_FACING = 0x400,
}

impl TraverseFlags {
  pub fn from_ray_flag(ray_flag: u32) -> Self {
    unsafe { std::mem::transmute(ray_flag) }
  }

  // todo check override priority
  pub fn merge_geometry_instance_flag(
    mut ray_flag: TraverseFlags,
    geometry_instance_flag: GeometryInstanceFlags,
  ) -> TraverseFlags {
    fn if_bit(
      source: GeometryInstanceFlags,
      bit: u32,
      flag: &mut TraverseFlags,
      if_true: impl FnOnce(u32) -> u32,
    ) {
      if source & bit > 0 {
        *flag = unsafe { std::mem::transmute(if_true(*flag as u32)) };
      }
    }

    use TraverseFlags::*;

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_TRIANGLE_FACING_CULL_DISABLE,
      &mut ray_flag,
      |flag| flag & !(CULL_BACK_FACING_TRIANGLES as u32 | CULL_FRONT_FACING_TRIANGLES as u32),
    );

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_TRIANGLE_FLIP_FACING,
      &mut ray_flag,
      |flag| flag ^ TRIANGLE_FLIP_FACING as u32,
    );

    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_FORCE_OPAQUE,
      &mut ray_flag,
      |flag| flag | FORCE_OPAQUE as u32,
    );
    if_bit(
      geometry_instance_flag,
      GEOMETRY_INSTANCE_FORCE_NO_OPAQUE,
      &mut ray_flag,
      |flag| flag | FORCE_NON_OPAQUE as u32,
    );

    ray_flag
  }

  /// returns Pass(true)/Fail(false), Opaque(true)/Non-opaque(false)
  pub fn cull_geometry(traverse_flag: TraverseFlags, geometry_flag: GeometryFlags) -> (bool, bool) {
    use TraverseFlags::*;

    let geometry_opaque = geometry_flag & GEOMETRY_FLAG_OPAQUE > 0;
    let force_opaque = traverse_flag as u32 & FORCE_OPAQUE as u32 > 0;
    let force_non_opaque = traverse_flag as u32 & FORCE_NON_OPAQUE as u32 > 0;
    let cull_opaque = traverse_flag as u32 & CULL_OPAQUE as u32 > 0;
    let cull_non_opaque = traverse_flag as u32 & CULL_NON_OPAQUE as u32 > 0;

    let is_opaque = (geometry_opaque || force_opaque) && !force_non_opaque;
    let pass = (is_opaque && !cull_opaque) || (!is_opaque && !cull_non_opaque);

    (pass, is_opaque)
  }

  /// returns CullEnable(true)/Disable(false), CullBack(true)/CullFront(false)
  pub fn cull_triangle(traverse_flag: TraverseFlags) -> (bool, bool) {
    use TraverseFlags::*;
    let flag = traverse_flag;
    let flip = flag as u32 & TRIANGLE_FLIP_FACING as u32 > 0;
    let cull_front = flag as u32 & CULL_FRONT_FACING_TRIANGLES as u32 > 0;
    let cull_back = flag as u32 & CULL_BACK_FACING_TRIANGLES as u32 > 0;

    let cull_enable = cull_front || cull_back;
    let cull_back = (flip && cull_back) || (!flip && cull_front);

    (cull_enable, cull_back)
  }

  pub fn visit_triangles(self) -> bool {
    (self as u32 & TraverseFlags::SKIP_TRIANGLES as u32) == 0
  }
  // pub fn visit_boxes(self) -> bool {
  //   (self as u32 & TraverseFlags::SKIP_BOXES as u32) == 0
  // }

  pub fn end_search_on_hit(self) -> bool {
    (self as u32 & TraverseFlags::ACCEPT_FIRST_HIT_AND_END_SEARCH as u32) > 0
  }
}

#[derive(Copy, Clone)]
pub struct TraverseFlagsGpu {
  flags: Node<u32>,
}

impl TraverseFlagsGpu {
  pub fn from_ray_flag(ray_flag: Node<u32>) -> Self {
    Self { flags: ray_flag }
  }
  pub fn as_u32(self) -> Node<u32> {
    self.flags
  }

  #[must_use]
  pub fn merge_geometry_instance_flag(self, geometry_instance_flag: Node<u32>) -> Self {
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

    use TraverseFlags::*;
    let traverse_flag = self.flags.make_local_var();

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

    Self::from_ray_flag(traverse_flag.load())
  }

  /// returns Pass(true)/Fail(false), Opaque(true)/Non-opaque(false)
  pub fn cull_geometry(self, geometry_flag: Node<u32>) -> (Node<bool>, Node<bool>) {
    use TraverseFlags::*;

    let flag = self.flags;
    let geometry_opaque = (geometry_flag & val(GEOMETRY_FLAG_OPAQUE)).greater_than(val(0));
    let force_opaque = (flag & val(FORCE_OPAQUE as u32)).greater_than(val(0));
    let force_non_opaque = (flag & val(FORCE_NON_OPAQUE as u32)).greater_than(val(0));
    let cull_opaque = (flag & val(CULL_OPAQUE as u32)).greater_than(0);
    let cull_non_opaque = (flag & val(CULL_NON_OPAQUE as u32)).greater_than(0);

    // write IS_OPAQUE
    let is_opaque = geometry_opaque.or(force_opaque).and(force_non_opaque.not());
    let pass = is_opaque
      .and(cull_opaque.not())
      .or(is_opaque.not().and(cull_non_opaque.not()));

    (pass, is_opaque)
  }

  /// returns CullEnable(true)/Disable(false), CullBack(true)/CullFront(false)
  pub fn cull_triangle(self) -> (Node<bool>, Node<bool>) {
    use TraverseFlags::*;
    let flag = self.flags;
    let flip = (flag & val(TRIANGLE_FLIP_FACING as u32)).greater_than(val(0));
    let cull_front = (flag & val(CULL_FRONT_FACING_TRIANGLES as u32)).greater_than(val(0));
    let cull_back = (flag & val(CULL_BACK_FACING_TRIANGLES as u32)).greater_than(val(0));

    let cull_enable = cull_front.or(cull_back);
    let cull_back = flip.and(cull_back).or(flip.not().and(cull_front));

    (cull_enable, cull_back)
  }

  pub fn visit_triangles(self) -> Node<bool> {
    (self.flags & val(TraverseFlags::SKIP_TRIANGLES as u32)).equals(val(0))
  }
  // pub fn visit_boxes(self) -> Node<bool> {
  //   (self.flags & val(TraverseFlags::SKIP_BOXES as u32)).equals(val(0))
  // }
  pub fn end_search_on_hit(self) -> Node<bool> {
    (self.flags & val(TraverseFlags::ACCEPT_FIRST_HIT_AND_END_SEARCH as u32)).greater_than(val(0))
  }
}
