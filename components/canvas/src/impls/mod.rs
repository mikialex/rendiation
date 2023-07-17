#![allow(dead_code)]
#![allow(unused)]

use lyon::{lyon_tessellation::StrokeTessellator, path::traits::PathBuilder};

use crate::*;

struct PainterCtx {
  recording: GraphicsRepresentation,
  transform_stack: Vec<TransformState>,
  masking_stack: Vec<GraphicsRepresentation>,
  filter_stack: Vec<CanvasEffect>,
}

#[derive(Debug, Clone, Copy)]
struct GraphicsVertex {
  position: Vec2<f32>,
  color: Vec3<f32>,
  uv: Vec2<f32>,
  object_id: u32, // point to ObjectMetaData[]
}

#[derive(Debug, Clone, Copy)]
struct ObjectMetaData {
  world_transform: Mat3<f32>,
  uv_base: Vec3<f32>,
}

struct TransformState {
  local: Mat3<f32>,
  world_computed: Mat3<f32>,
}

#[derive(Default)]
struct GraphicsRepresentation {
  object_meta: Vec<ObjectMetaData>,
  vertices: Vec<GraphicsVertex>,
  indices: Vec<u32>,

  images: Vec<GraphicsImageData>,
}

impl PainterCtx {
  fn get_current_world_transform(&self) -> Mat3<f32> {
    self
      .transform_stack
      .last()
      .map(|v| v.world_computed)
      .unwrap_or(Mat3::identity())
  }
}

enum GraphicsImageData {
  Bitmap,
  Gpu,
}

impl PainterAPI for PainterCtx {
  fn reset(&mut self) {
    todo!()
  }

  type Image = GraphicsImageData;

  fn register_image(&mut self, image: Self::Image) -> TextureHandle {
    let handle = self.recording.images.len();
    self.recording.images.push(image);
    handle
  }

  type Baked = GraphicsRepresentation;

  fn draw_bake(&mut self, p: &Self::Baked) {
    //
  }

  fn bake(self) -> Self::Baked {
    self.recording
  }
  fn render(&self) -> Self::Image {
    todo!()
  }

  fn stroke_shape(&mut self, shape: &Shape, style: &StrokeStyle) {
    let world_transform = self.get_current_world_transform();
    let meta = ObjectMetaData {
      world_transform,
      uv_base: Default::default(),
    };

    let builder = MeshBuilder::new(&mut self.recording.vertices, &mut self.recording.indices);
    triangulate_stroke(shape, style, builder);
  }

  fn fill_shape(&mut self, shape: &Shape, style: &FillStyle) {
    let world_transform = self.get_current_world_transform();
    let meta = ObjectMetaData {
      world_transform,
      uv_base: Default::default(),
    };

    let builder = MeshBuilder::new(&mut self.recording.vertices, &mut self.recording.indices);
    triangulate_fill(shape, style, builder);
  }

  fn push_transform(&mut self, transform: Mat3<f32>) {
    let world_computed = transform * self.get_current_world_transform();
    self.transform_stack.push(TransformState {
      local: transform,
      world_computed,
    })
  }

  fn pop_transform(&mut self) -> Option<Mat3<f32>> {
    self.transform_stack.pop().map(|v| v.local)
  }

  fn push_mask(&mut self, mask: Self::Baked) {
    self.masking_stack.push(mask)
  }

  fn pop_mask(&mut self) -> Option<Self::Baked> {
    self.masking_stack.pop()
  }

  fn push_filter(&mut self, effect: CanvasEffect) {
    self.filter_stack.push(effect)
  }

  fn pop_filter(&mut self) -> Option<CanvasEffect> {
    self.filter_stack.pop()
  }
}

// todo, handle max u32
struct MeshBuilder<'a> {
  vertices: &'a mut Vec<GraphicsVertex>,
  indices: &'a mut Vec<u32>,
}

impl<'a> MeshBuilder<'a> {
  pub fn new(vertices: &'a mut Vec<GraphicsVertex>, indices: &'a mut Vec<u32>) -> Self {
    let start = vertices.len() as u32;
    Self { vertices, indices }
  }
}

impl<'a> lyon::lyon_tessellation::GeometryBuilder for MeshBuilder<'a> {
  fn add_triangle(
    &mut self,
    a: lyon::lyon_tessellation::VertexId,
    b: lyon::lyon_tessellation::VertexId,
    c: lyon::lyon_tessellation::VertexId,
  ) {
    self.indices.push(a.offset());
    self.indices.push(b.offset());
    self.indices.push(c.offset());
  }
}

impl<'a> lyon::lyon_tessellation::FillGeometryBuilder for MeshBuilder<'a> {
  fn add_fill_vertex(
    &mut self,
    vertex: lyon::lyon_tessellation::FillVertex,
  ) -> Result<lyon::lyon_tessellation::VertexId, lyon::lyon_tessellation::GeometryBuilderError> {
    // todo uv color
    let position = vertex.position();
    let vertex = GraphicsVertex {
      position: Vec2::new(position.x, position.y),
      color: Vec3::zero(),
      uv: Vec2::zero(),
      object_id: 0,
    };
    let index = self.vertices.len();
    self.vertices.push(vertex);
    Ok(lyon::lyon_tessellation::VertexId::from(index as u32))
  }
}

impl<'a> lyon::lyon_tessellation::StrokeGeometryBuilder for MeshBuilder<'a> {
  fn add_stroke_vertex(
    &mut self,
    vertex: lyon::lyon_tessellation::StrokeVertex,
  ) -> Result<lyon::lyon_tessellation::VertexId, lyon::lyon_tessellation::GeometryBuilderError> {
    // todo uv color
    let position = vertex.position();
    let vertex = GraphicsVertex {
      position: Vec2::new(position.x, position.y),
      color: Vec3::zero(),
      uv: Vec2::zero(),
      object_id: 0,
    };
    let index = self.vertices.len();
    self.vertices.push(vertex);
    Ok(lyon::lyon_tessellation::VertexId::from(index as u32))
  }
}

fn triangulate_stroke(
  shape: &Shape,
  style: &StrokeStyle,
  mut builder: impl lyon::lyon_tessellation::StrokeGeometryBuilder,
) {
  use lyon::tessellation::{StrokeOptions, StrokeTessellator};

  let options = StrokeOptions::tolerance(0.1);
  let mut tessellator = StrokeTessellator::new();

  let mut builder = tessellator.builder(&options, &mut builder);

  match shape {
    Shape::Rect(rect) => {
      builder.add_rectangle(&into_lyon_rect(rect), lyon::path::Winding::Positive)
    }
    Shape::RoundCorneredRect(round_rect) => builder.add_rounded_rectangle(
      &into_lyon_rect(&round_rect.rect),
      &into_lyon_radius(&round_rect.radius),
      lyon::path::Winding::Positive,
    ),
    Shape::Path(path) => {
      for seg in &path.sub_paths {
        builder.begin(into_lyon_point(seg.start));
        for p in &seg.paths {
          match &p.path {
            Path2dType::Line(_) => {
              builder.line_to(into_lyon_point(p.end_point));
            }
            Path2dType::QuadraticBezier(c) => {
              builder.quadratic_bezier_to(into_lyon_point(c.ctrl), into_lyon_point(p.end_point));
            }
            Path2dType::CubicBezier(c) => {
              builder.cubic_bezier_to(
                into_lyon_point(c.ctrl1),
                into_lyon_point(c.ctrl2),
                into_lyon_point(p.end_point),
              );
            }
          }
        }
        builder.end(seg.closed)
      }
    }
  }

  builder.build();
}

fn triangulate_fill(
  shape: &Shape,
  style: &FillStyle,
  mut builder: impl lyon::lyon_tessellation::FillGeometryBuilder,
) {
  use lyon::tessellation::{FillOptions, FillTessellator};

  let options = FillOptions::tolerance(0.1);
  let mut tessellator = FillTessellator::new();

  let mut builder = tessellator.builder(&options, &mut builder);

  match shape {
    Shape::Rect(rect) => {
      builder.add_rectangle(&into_lyon_rect(rect), lyon::path::Winding::Positive)
    }
    Shape::RoundCorneredRect(round_rect) => builder.add_rounded_rectangle(
      &into_lyon_rect(&round_rect.rect),
      &into_lyon_radius(&round_rect.radius),
      lyon::path::Winding::Positive,
    ),
    Shape::Path(path) => {
      for seg in &path.sub_paths {
        builder.begin(into_lyon_point(seg.start));
        for p in &seg.paths {
          match &p.path {
            Path2dType::Line(_) => {
              builder.line_to(into_lyon_point(p.end_point));
            }
            Path2dType::QuadraticBezier(c) => {
              builder.quadratic_bezier_to(into_lyon_point(c.ctrl), into_lyon_point(p.end_point));
            }
            Path2dType::CubicBezier(c) => {
              builder.cubic_bezier_to(
                into_lyon_point(c.ctrl1),
                into_lyon_point(c.ctrl2),
                into_lyon_point(p.end_point),
              );
            }
          }
        }
        builder.end(seg.closed)
      }
    }
  }

  builder.build();
}

fn into_lyon_point(v: Vec2<f32>) -> lyon::math::Point {
  lyon::math::point(v.x, v.y)
}

fn into_lyon_rect(rect: &RectangleShape) -> lyon::math::Box2D {
  lyon::math::Box2D {
    min: lyon::math::point(rect.x, rect.y),
    max: lyon::math::point(rect.x + rect.width, rect.y + rect.height),
  }
}

fn into_lyon_radius(radius: &RadiusGroup) -> lyon::path::builder::BorderRadii {
  lyon::path::builder::BorderRadii {
    top_left: radius.top_left,
    top_right: radius.top_right,
    bottom_left: radius.bottom_left,
    bottom_right: radius.bottom_right,
  }
}
