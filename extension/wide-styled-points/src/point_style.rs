use crate::*;

pub fn wide_line_vertex(
  logic_point_position: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  view_size: Node<Vec2<f32>>,
  width: Node<f32>,
  builder: &mut ShaderVertexBuilder,
) {
  let object_world_position = builder.query::<WorldPositionHP>();
  let (clip, _) = camera_transform_impl(builder, logic_point_position, object_world_position);

  let ndc = clip.xyz() / clip.w().splat();

  let width: Node<Vec2<f32>> = width.splat::<Vec2<f32>>() / view_size;

  let position = position.xy() * val(Vec2::new(2., 2.)) - val(Vec2::splat(1.));

  let offset = position * width;
  let offset = (offset, val(0.)).into();

  let clip: Node<Vec4<f32>> = (ndc + offset, val(1.)).into();

  builder.register::<ClipPosition>(clip);

  // this should be optional(current used for clip effect)
  {
    let view_proj_inv = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
    let position = view_proj_inv * clip;
    let position = position.xyz() / position.w().splat();
    builder.register::<VertexRenderPosition>(position);
  }
}

only_vertex!(WidePointPosition, Vec3<f32>);
only_vertex!(WidePointSize, f32);
only_vertex!(WidePointStyleId, u32);

fn point_style_entry(p: Node<Vec2<f32>>, style_type: Node<u32>) -> Node<f32> {
  todo!()
}

// we use +-1 as point coord
fn point_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let x = p.x().abs() - val(0.8);
  let y = p.y().abs() - val(0.8);
  x.max(y)
}

fn plus_style(p: Node<Vec2<f32>>) -> Node<f32> {
  p.x().abs().min(p.y().abs())
}

fn sd_segment(p: Node<Vec2<f32>>, start: Node<Vec2<f32>>, end: Node<Vec2<f32>>) -> Node<f32> {
  todo!()
}

fn v2_node(x: f32, y: f32) -> Node<Vec2<f32>> {
  val(Vec2::new(x, y))
}

fn start_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment(p, v2_node(-0.5, 0.3), v2_node(0.5, -0.3));
  let d2 = sd_segment(p, v2_node(-0.5, -0.3), v2_node(0.5, 0.3));
  let d3 = sd_segment(p, v2_node(0.0, -0.6), v2_node(0.0, 0.6));
  d1.min(d2).min(d3)
}

fn cross_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment(p, v2_node(-0.7, 0.7), v2_node(0.7, -0.7));
  let d2 = sd_segment(p, v2_node(-0.7, -0.7), v2_node(0.7, 0.7));
  d1.min(d2)
}

fn circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  (p.length() - val(0.85)).abs()
}

fn point_circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  todo!()
}
