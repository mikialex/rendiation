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
both!(WidePointStyleId, u32);
both!(WidePointTextureId, Vec2<u32>);

pub fn point_style_entry(
  p: Node<Vec2<f32>>,
  style_type: Node<u32>,
) -> (Node<f32>, Node<Vec3<f32>>) {
  let result = val(1.).make_local_var();
  let color_multiplier = val(Vec3::new(1., 1., 1.)).make_local_var();
  switch_by(style_type)
    .case(0, || result.store(point_style(p, val(0.8))))
    .case(1, || result.store(plus_style(p)))
    .case(2, || result.store(start_style(p)))
    .case(3, || result.store(cross_style(p)))
    .case(4, || result.store(circle_style(p, val(0.85))))
    .case(5, || result.store(point_circle_style(p)))
    .case(6, || result.store(plus_circle_style(p)))
    .case(7, || result.store(start_circle_style(p)))
    .case(8, || result.store(cross_circle_style(p)))
    .case(9, || result.store(ring_style(p, val(0.9), val(0.3))))
    .case(10, || result.store(ring_style(p, val(0.9), val(0.45))))
    .case(11, || result.store(ring_style(p, val(0.9), val(0.7))))
    .case(12, || {
      result.store(sphere_style(p));
      let c = val(0.8) * (val(1.0) - p.length());
      color_multiplier.store(c.splat());
    })
    .case(13, || result.store(sphere_style(p)))
    .case(14, || result.store(square_style(p)))
    .case(15, || result.store(diamond_style(p)))
    .end_with_default(|| {});

  let alpha = val(1.) - result.load().smoothstep(0.0, 0.1);

  (alpha, color_multiplier.load())
}

// we use +-1 as point coord
fn point_style(p: Node<Vec2<f32>>, ratio: Node<f32>) -> Node<f32> {
  let x = p.x().abs() - ratio;
  let y = p.y().abs() - ratio;
  x.max(y)
}

fn plus_style(p: Node<Vec2<f32>>) -> Node<f32> {
  p.x().abs().min(p.y().abs())
}

#[shader_fn]
fn sd_segment(p: Node<Vec2<f32>>, a: Node<Vec2<f32>>, b: Node<Vec2<f32>>) -> Node<f32> {
  let ba = b - a;
  let pa = p - a;
  let c1 = ba.dot(pa);
  let c2 = ba.dot(ba);
  let h = (c1 / c2).saturate();
  (pa - ba * h).length()
}

fn v2_node(x: f32, y: f32) -> Node<Vec2<f32>> {
  val(Vec2::new(x, y))
}

fn start_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment_fn(p, v2_node(-0.5, 0.3), v2_node(0.5, -0.3));
  let d2 = sd_segment_fn(p, v2_node(-0.5, -0.3), v2_node(0.5, 0.3));
  let d3 = sd_segment_fn(p, v2_node(0.0, -0.6), v2_node(0.0, 0.6));
  d1.min(d2).min(d3)
}

fn cross_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment_fn(p, v2_node(-0.7, 0.7), v2_node(0.7, -0.7));
  let d2 = sd_segment_fn(p, v2_node(-0.7, -0.7), v2_node(0.7, 0.7));
  d1.min(d2)
}

fn circle_style(p: Node<Vec2<f32>>, radius: Node<f32>) -> Node<f32> {
  (p.length() - radius).abs()
}

fn point_circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let point = point_style(p, val(0.1));
  let circle = circle_style(p, val(0.85));
  point.min(circle)
}

fn plus_circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment_fn(p, v2_node(0.0, -0.3), v2_node(0.0, 0.3));
  let d2 = sd_segment_fn(p, v2_node(-0.3, 0.0), v2_node(0.3, 0.0));
  let plus = d1.min(d2);

  let circle = circle_style(p, val(0.85));
  plus.min(circle)
}

fn start_circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment_fn(p, v2_node(-0.25, 0.15), v2_node(0.25, -0.15));
  let d2 = sd_segment_fn(p, v2_node(-0.25, -0.15), v2_node(0.25, 0.15));
  let d3 = sd_segment_fn(p, v2_node(0.0, -0.3), v2_node(0.0, 0.3));
  let start = d1.min(d2).min(d3);

  let circle = circle_style(p, val(0.85));
  start.min(circle)
}

fn cross_circle_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let d1 = sd_segment_fn(p, v2_node(-0.25, 0.25), v2_node(0.25, -0.25));
  let d2 = sd_segment_fn(p, v2_node(-0.25, -0.25), v2_node(0.25, 0.25));
  let cross = d1.min(d2);

  let circle = circle_style(p, val(0.85));
  cross.min(circle)
}

fn ring_style(p: Node<Vec2<f32>>, out_radius: Node<f32>, inner_radius: Node<f32>) -> Node<f32> {
  let d = p.length();
  let ring_dist = d - out_radius;
  let inner_edge = inner_radius - d;
  ring_dist.max(inner_edge)
}

fn sphere_style(p: Node<Vec2<f32>>) -> Node<f32> {
  p.length() - val(0.85)
}

fn square_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let r = 0.85;
  let d1 = sd_segment_fn(p, v2_node(-r, r), v2_node(r, r));
  let d2 = sd_segment_fn(p, v2_node(-r, r), v2_node(-r, -r));
  let d3 = sd_segment_fn(p, v2_node(r, -r), v2_node(r, r));
  let d4 = sd_segment_fn(p, v2_node(r, -r), v2_node(-r, -r));
  d1.min(d2).min(d3).min(d4)
}

fn diamond_style(p: Node<Vec2<f32>>) -> Node<f32> {
  let r = 0.85;
  let d1 = sd_segment_fn(p, v2_node(0.0, r), v2_node(r, 0.0));
  let d2 = sd_segment_fn(p, v2_node(0.0, r), v2_node(-r, 0.0));
  let d3 = sd_segment_fn(p, v2_node(0.0, -r), v2_node(r, 0.0));
  let d4 = sd_segment_fn(p, v2_node(0.0, -r), v2_node(-r, 0.0));
  d1.min(d2).min(d3).min(d4)
}
