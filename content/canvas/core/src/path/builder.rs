use crate::*;

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

// pub struct Path2dBuilder {
//   path: Vec<Path2dSegments>,
//   current_point: Vec2<f32>,
// }

// impl Default for Path2dBuilder {
//   fn default() -> Self {
//     Self {
//       path: Default::default(),
//       current_point: Vec2::new(0., 0.),
//     }
//   }
// }

// impl Path2dBuilder {
//   pub fn line_to(&mut self, point: impl Into<Vec2<f32>>) -> &mut Self {
//     let point = point.into();
//     let curve = LineSegment::line_segment(self.current_point, point);
//     self.path.push(Path2dSegments::Line(curve));
//     self.current_point = point;
//     self
//   }

//   pub fn move_to(&mut self, point: impl Into<Vec2<f32>>) -> &mut Self {
//     self.current_point = point.into();
//     self
//   }

//   fn close_path(&mut self) {
//     // check should close?
//     if self.path.len() <= 2 {
//       return;
//     }

//     let start = self.path.first().unwrap().start();
//     let end = self.path.last().unwrap().end();

//     // check if has closed actually
//     if start != end {
//       self
//         .path
//         .push(Path2dSegments::Line(LineSegment::line_segment(end, start)));
//     }
//   }

//   pub fn build(mut self, close_path: bool) -> Path2dSegments<f32> {
//     if close_path {
//       self.close_path();
//     }

//     Path2dSegments {
//       segments: self.path,
//     }
//   }
// }
