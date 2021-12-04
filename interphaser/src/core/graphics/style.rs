pub struct FillStyle {
  color: Color,
  alpha: f32,
}

pub struct StrokeStyle {
  width: f32,
  line_cap: LineCap,
  line_join: LineJoin,
}

pub enum LineCap {
  Butt,
  Square,
  Round,
}

pub enum LineJoin {
  Miter,
  Round,
  Bevel,
}
