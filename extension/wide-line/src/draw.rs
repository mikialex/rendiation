use std::ops::BitAnd;

use crate::*;

only_vertex!(WideLineStart, Vec3<f32>);
only_vertex!(WideLineEnd, Vec3<f32>);
only_vertex!(WideLinePosition, Vec3<f32>);
only_vertex!(WideLineVertexColor, u32);
only_vertex!(WideLineStartColor, u32);
only_vertex!(WideLineEndColor, u32);

/// the quad vertex position y less than 0.5 is at the start side of the segment
pub fn wide_line_vertex_color(
  position: Node<Vec3<f32>>,
  start_color: Node<u32>,
  end_color: Node<u32>,
) -> Node<Vec4<f32>> {
  position
    .y()
    .less_than(0.5)
    .select(start_color, end_color)
    .unpack4x8unorm()
}

/// trim the segment if one endpoint is behind or too close to the camera plane, the projection
/// of such endpoint to ndc is invalid(w <= 0). the trimmed endpoint is placed between the camera
/// plane and the near plane like three.js, so it(and its end cap) is still clipped by hardware.
fn trim_segment_by_near_plane(
  projection: Node<Mat4<f32>>,
  clip_start: Node<Vec4<f32>>,
  clip_end: Node<Vec4<f32>>,
) -> (Node<Vec4<f32>>, Node<Vec4<f32>>) {
  // the near plane is z = 0 for standard depth, and z = w for reversed depth
  let a = projection.z().z();
  let b = projection.w().z();
  let near = b / a.less_than(0.).select(a, val(1.) + a);
  let target_w = near * val(0.5);

  let is_perspective = projection.z().w().less_than(-0.5);
  let start_w = clip_start.w();
  let end_w = clip_end.w();

  let trim_start = is_perspective
    .and(start_w.less_than(target_w))
    .and(end_w.greater_equal_than(target_w));
  let trim_end = is_perspective
    .and(end_w.less_than(target_w))
    .and(start_w.greater_equal_than(target_w));

  let t_start = (target_w - start_w) / (end_w - start_w);
  let t_end = (target_w - end_w) / (start_w - end_w);

  let clip_start_trimmed = trim_start.select(t_start.mix(clip_start, clip_end), clip_start);
  let clip_end_trimmed = trim_end.select(t_end.mix(clip_end, clip_start), clip_end);
  (clip_start_trimmed, clip_end_trimmed)
}

/// the screen coord is only used for computing the major axis of line pattern by derivatives
pub fn set_wide_line_screen_coord(builder: &mut ShaderVertexBuilder, clip: Node<Vec4<f32>>) {
  let sc = clip.xy() / clip.w();
  let sc = sc * val(Vec2::splat(0.5)) + val(Vec2::splat(0.5));
  let viewport_size = builder.query::<ViewportRenderBufferSize>();
  builder.set_vertex_out::<WideLineScreenCoord>(sc * viewport_size);
}

pub fn wide_line_vertex(
  wide_line_start: Node<Vec3<f32>>,
  wide_line_end: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  view_size: Node<Vec2<f32>>,
  width: Node<f32>,
  builder: &mut ShaderVertexBuilder,
) {
  let object_world_position = builder.query::<WorldPositionHP>();
  let (clip_start, _) = camera_transform_impl(builder, wide_line_start, object_world_position);
  let (clip_end, _) = camera_transform_impl(builder, wide_line_end, object_world_position);

  let projection = builder.query::<CameraProjectionMatrix>();
  let (clip_start, clip_end) = trim_segment_by_near_plane(projection, clip_start, clip_end);

  let aspect = view_size.x() / view_size.y();

  // ndc space
  let ndc_start = clip_start.xy() / clip_start.w();
  let ndc_end = clip_end.xy() / clip_end.w();

  // direction
  let dir = ndc_end - ndc_start;

  // account for clip-space aspect ratio
  let dir = vec2_node((dir.x() * aspect, dir.y()));
  let dir = dir.normalize();

  // perpendicular to dir
  let offset = vec2_node((dir.y(), -dir.x()));

  // undo aspect ratio adjustment
  let dir = vec2_node((dir.x() / aspect, dir.y()));
  let offset = vec2_node((offset.x() / aspect, offset.y()));
  let offset = offset.make_local_var();

  // sign flip
  if_by(position.x().less_than(0.), || {
    offset.store(-offset.load());
  });

  // end caps
  if_by(position.y().less_than(0.), || {
    offset.store(offset.load() - dir);
  });

  if_by(position.y().greater_than(1.), || {
    offset.store(offset.load() + dir);
  });

  let mut offset = offset.load();

  // adjust for width
  offset *= width;
  // adjust for clip-space to screen-space conversion
  // maybe resolution should be based on viewport ...
  offset /= view_size.y();

  // select end
  let clip_no_offset = position.y().less_than(0.5).select(clip_start, clip_end);

  // back to clip space
  offset *= clip_no_offset.w();
  let clip: Node<Vec4<f32>> = (clip_no_offset.xy() + offset, clip_no_offset.zw()).into();

  builder.register::<ClipPosition>(clip);

  // for line pattern effect
  set_wide_line_screen_coord(builder, clip_no_offset);

  // this should be optional(current used for clip effect)
  {
    let view_proj_inv = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
    let position = view_proj_inv * clip;
    let position = position.xyz() / position.w();
    builder.register::<VertexRenderPosition>(position);
  }
}

#[shader_fn]
pub fn discard_by_round_corner(uv: Node<Vec2<f32>>) -> Node<bool> {
  let a = uv.x();
  let b = uv.y() + uv.y().greater_than(0.).select(-1., 1.);
  let len2 = a * a + b * b;

  uv.y().abs().greater_than(1.).and(len2.greater_than(1.))
}

both!(WideLineScreenCoord, Vec2<f32>);

#[shader_fn]
pub fn discard_by_line_pattern(
  factor: Node<f32>,
  pattern: Node<u32>,
  screen_coord: Node<Vec2<f32>>,
  frag_position: Node<Vec2<f32>>,
) -> Node<bool> {
  let x_ratio = screen_coord.x().dpdx_fine().abs();
  let y_ratio = screen_coord.y().dpdy_fine().abs();

  let an_axis = (x_ratio - y_ratio)
    .greater_than(0.001)
    .select(val(Vec2::new(1.0, 0.0)), val(Vec2::new(0.0, 1.0)));

  let rotate_point = frag_position.dot(an_axis);

  let a_bit = (rotate_point / factor + val(0.5))
    .floor()
    .into_u32()
    .bitand(val(15_u32));

  pattern.bitand(val(1_u32) << a_bit).equals(val(0))
}
