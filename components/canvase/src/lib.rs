use rendiation_algebra::*;

pub trait Canvas2DPathBuilder {
  fn move_to(&mut self, point: Vec2<f32>);

  fn line_to(&mut self, point: Vec2<f32>);

  fn quadratic_bezier_curve_to(&mut self, control_point: Vec2<f32>, end_point: Vec2<f32>);

  fn bezier_curve_to(
    &mut self,
    control_point1: Vec2<f32>,
    control_point2: Vec2<f32>,
    end_point: Vec2<f32>,
  );

  fn arc(
    &mut self,
    center: Vec2<f32>,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    is_counter_clock_wise: bool,
  );

  fn arc_to(&mut self, ctrl1: Vec2<f32>, ctrl2: Vec2<f32>, radius: f32);

  fn rect(&mut self, origin: Vec2<f32>, size: Vec2<f32>);
}
