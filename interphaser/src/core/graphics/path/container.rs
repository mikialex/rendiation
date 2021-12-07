use rendiation_algebra::{Scalar, Vec2};
use rendiation_geometry::LineSegment;

use crate::Path2dSegment;

pub struct Path2D<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

pub struct Path2DLoop<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

pub struct ConvexPath2DLoop<T> {
  pub segments: Vec<Path2dSegment<T>>,
}

pub struct MeshBuilder<T> {
  buffer: Vec<T>,
}

impl<T> MeshBuilder<T> {
  pub fn add_triangle(&mut self, a: T, b: T, c: T) {
    self.buffer.push(a);
    self.buffer.push(b);
    self.buffer.push(c);
  }
}

pub struct TessellatedPathBuffer<T> {
  segments: Vec<LineSegment<Vec2<T>>>,
  is_convex: bool,
}

impl<T: Scalar> TessellatedPathBuffer<T> {
  pub fn reset(&mut self) {
    self.segments.clear();
    self.is_convex = false;
  }

  pub fn add_line_segment(&mut self, line: LineSegment<Vec2<T>>) {
    self.segments.push(line);
  }

  pub fn triangulate_fill(&self, mesh_builder: &mut MeshBuilder<Vec2<T>>) {
    if self.is_convex {
      let start_point = self.segments[0].start;
      for segment in self.segments.get(1..).unwrap().iter() {
        mesh_builder.add_triangle(start_point, segment.start, segment.end);
      }
    } else {
      todo!()
    }
  }
}

impl<T: Scalar> ConvexPath2DLoop<T> {
  pub fn triangulate_fill(&self, buffer: &mut TessellatedPathBuffer<T>) {
    buffer.reset();
    buffer.is_convex = true;
    for segment in &self.segments {
      segment.tessellate(buffer)
    }
  }
}

/// collections of container for building path mesh, For reusing the memory and
/// avoid allocation
pub struct Path2dTesselationCtx {
  pub path_buffer: ConvexPath2DLoop<f32>,
  pub path_tesselate_buffer: TessellatedPathBuffer<f32>,
  pub mesh_build_buffer: MeshBuilder<Vec2<f32>>,
}
