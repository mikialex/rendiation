use crate::*;

only_vertex!(WideLineStart, Vec3<f32>);
only_vertex!(WideLineEnd, Vec3<f32>);

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

  let aspect = view_size.x() / view_size.y();

  // ndc space
  let ndc_start = clip_start.xy() / clip_start.w().splat();
  let ndc_end = clip_end.xy() / clip_end.w().splat();

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
  offset *= width.splat();
  // adjust for clip-space to screen-space conversion
  // maybe resolution should be based on viewport ...
  offset /= view_size.y().splat();

  // select end
  let clip = position.y().less_than(0.5).select(clip_start, clip_end);

  // back to clip space
  offset = offset * clip.w();
  let clip = (clip.xy() + offset, clip.zw()).into();

  builder.register::<ClipPosition>(clip);

  // this should be optional(current used for clip effect)
  {
    let view_proj_inv = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
    let position = view_proj_inv * clip;
    let position = position.xyz() / position.w().splat();
    builder.register::<VertexRenderPosition>(position);
  }
}

#[shader_fn]
pub fn discard_round_corner(uv: Node<Vec2<f32>>) -> Node<bool> {
  let a = uv.x();
  let b = uv.y() + uv.y().greater_than(0.).select(-1., 1.);
  let len2 = a * a + b * b;

  uv.y().abs().greater_than(1.).and(len2.greater_than(1.))
}
