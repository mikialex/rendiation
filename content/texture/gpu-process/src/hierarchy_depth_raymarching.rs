use crate::*;

/// Requires origin and direction of the ray to be in screen space [0, 1] x [0, 1]
pub fn hierarchical_raymarch(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  //   is_mirror: Node<bool>,
  screen_size: Node<Vec2<u32>>,
  most_detailed_mip: Node<i32>,
  max_traversal_intersections: Node<u32>,
  hierarch_depth: BindingNode<ShaderTexture2D>,
  reverse_depth: bool,
  max_mipmap_level: u32,
) -> (Node<Vec3<f32>>, Node<bool>) {
  let inv_direction = direction
    .not_equals(Vec3::zero())
    .all()
    .select(val(Vec3::one()) / direction, val(Vec3::splat(f32::MAX)));

  // Start on mip with highest detail.
  let current_mip = most_detailed_mip.make_local_var();

  // Could recompute these every iteration, but it's faster to hoist them out and update them.
  let current_mip_resolution = vec2_node((
    screen_size.x() >> current_mip.load().into_u32(),
    screen_size.y() >> current_mip.load().into_u32(),
  ))
  .into_f32();
  let current_mip_resolution_inv = val(Vec2::splat(1.)) / current_mip_resolution;

  // Offset to the bounding boxes uv space to intersect the ray with the center of the next pixel.
  // This means we ever so slightly over shoot into the next region.
  let uv_offset =
    (val(0.005) * (most_detailed_mip).exp2().into_f32()).splat() / screen_size.into_f32();
  let uv_offset = vec2_node((
    direction
      .x()
      .less_than(0.)
      .select(-uv_offset.x(), uv_offset.x()),
    direction
      .y()
      .less_than(0.)
      .select(-uv_offset.y(), uv_offset.y()),
  ));

  // Offset applied depending on current mip resolution to move the boundary to the left/right upper/lower border depending on ray direction.
  let floor_offset = vec2_node((
    direction.x().less_than(0.).select(0., 1.),
    direction.y().less_than(0.).select(0., 1.),
  ));

  // Initially advance ray to avoid immediate self intersections.
  let (position, current_t) = initial_advance_ray(
    origin,
    direction,
    inv_direction,
    current_mip_resolution,
    current_mip_resolution_inv,
    floor_offset,
    uv_offset,
  );

  let current_mip_resolution = current_mip_resolution.make_local_var();
  let current_mip_resolution_inv = current_mip_resolution_inv.make_local_var();
  let position = position.make_local_var();
  let current_t = current_t.make_local_var();

  //   let exit_due_to_low_occupancy = val(false).make_local_var();
  let i = val(0_u32).make_local_var();
  loop_by(|cx| {
    let break_condition = i
      .load()
      .less_than(max_traversal_intersections)
      .and(current_mip.load().greater_equal_than(most_detailed_mip));

    if_by(break_condition, || {
      cx.do_break();
    });

    let current_mip_position = current_mip_resolution.load() * position.load().xy();
    let surface_z = hierarch_depth
      .load_texel(
        current_mip_position.into_u32(),
        current_mip.load().into_u32(),
      )
      .x();
    //  exit_due_to_low_occupancy = !is_mirror && ffxWaveActiveCountBits(true) <= min_traversal_occupancy;
    let skipped_tile = advance_ray(
      origin,
      direction,
      inv_direction,
      current_mip_position,
      current_mip_resolution_inv.load(),
      floor_offset,
      uv_offset,
      surface_z,
      position.clone(),
      current_t.clone(),
      reverse_depth,
    );

    // Don't increase mip further than this because we did not generate it
    let next_mip_is_out_of_range = skipped_tile.and(
      current_mip
        .load()
        .greater_equal_than(max_mipmap_level as i32),
    );

    if_by(next_mip_is_out_of_range.not(), || {
      current_mip.store(current_mip.load() + skipped_tile.select(1, -1));
      current_mip_resolution
        .store(current_mip_resolution.load() * skipped_tile.select(val(0.5), val(2.0)));
      current_mip_resolution_inv
        .store(current_mip_resolution_inv.load() * skipped_tile.select(val(2.0), val(0.5)));
    });

    i.store(i.load() + val(1));
  });

  let valid_hit = i.load().less_equal_than(max_traversal_intersections);

  (position.load(), valid_hit)
}

/// return (position, current_t)
fn initial_advance_ray(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  inv_direction: Node<Vec3<f32>>,
  current_mip_resolution: Node<Vec2<f32>>,
  current_mip_resolution_inv: Node<Vec2<f32>>,
  floor_offset: Node<Vec2<f32>>,
  uv_offset: Node<Vec2<f32>>,
) -> (Node<Vec3<f32>>, Node<f32>) {
  let current_mip_position = current_mip_resolution * origin.xy();

  // Intersect ray with the half box that is pointing away from the ray origin.
  let xy_plane = current_mip_position.floor() + floor_offset;
  let xy_plane = xy_plane * current_mip_resolution_inv + uv_offset;

  // o + d * t = p' => t = (p' - o) / d
  let t = xy_plane * inv_direction.xy() - origin.xy() * inv_direction.xy();
  let current_t = t.x().min(t.y());
  let position = origin + current_t * direction;
  (position, current_t)
}

fn advance_ray(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  inv_direction: Node<Vec3<f32>>,
  current_mip_position: Node<Vec2<f32>>,
  current_mip_resolution_inv: Node<Vec2<f32>>,
  floor_offset: Node<Vec2<f32>>,
  uv_offset: Node<Vec2<f32>>,
  surface_z: Node<f32>,
  position: ShaderPtrOf<Vec3<f32>>,
  current_t: ShaderPtrOf<f32>,
  reverse_depth: bool,
) -> Node<bool> {
  // Create boundary planes
  let xy_plane = current_mip_position.floor() + floor_offset;
  let xy_plane = xy_plane * current_mip_resolution_inv + uv_offset;
  let boundary_planes = vec3_node((xy_plane, surface_z));

  // Intersect ray with the half box that is pointing away from the ray origin.
  // o + d * t = p' => t = (p' - o) / d
  let t = boundary_planes * inv_direction - origin * inv_direction;

  // Prevent using z plane when shooting out of the depth buffer.
  let z = if reverse_depth {
    direction.z().less_than(0.)
  } else {
    direction.z().greater_than(0.)
  }
  .select(t.z(), f32::MAX);
  let t = vec3_node((t.x(), t.y(), z));

  // Choose nearest intersection with a boundary.
  let t_min = t.x().min(t.y()).min(t.z());

  let above_surface = if reverse_depth {
    surface_z.less_than(position.load().z())
  } else {
    surface_z.greater_than(position.load().z())
  };

  // Decide whether we are able to advance the ray until we hit the xy boundaries or if we had to clamp it at the surface.
  // We use the asuint comparison to avoid NaN / Inf logic, also we actually care about bitwise equality here to see if t_min is the t.z we fed into the min3 above.
  let skipped_tile = t_min
    .into_u32()
    .not_equals(t.z().into_u32())
    .and(above_surface);

  // Make sure to only advance the ray if we're still above the surface.
  current_t.store(above_surface.select(t_min, current_t.load()));

  // Advance ray
  position.store(origin + current_t.load() * direction);

  skipped_tile
}
