#![allow(dead_code)]
#![allow(unused)]

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
}

struct TransformState {
  local: Mat3<f32>,
  world_computed: Mat3<f32>,
}

#[derive(Default)]
struct GraphicsRepresentation {
  object_meta: Vec<ObjectMetaData>,
  triangulated: Vec<GraphicsVertex>,

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
    todo!()
  }

  fn bake(self) -> Self::Baked {
    self.recording
  }
  fn render(&self) -> Self::Image {
    todo!()
  }

  fn draw_baked(&mut self, baked: Self::Baked) {
    todo!()
  }

  fn stroke_shape(&mut self, shape: &Shape, style: &StrokeStyle) {
    let world_transform = self.get_current_world_transform();
    let meta = ObjectMetaData { world_transform };

    triangulate_stroke(shape, style, |v| {
      self.recording.triangulated.push(v);
    });
  }

  fn fill_shape(&mut self, shape: &Shape, style: &FillStyle) {
    let world_transform = self.get_current_world_transform();
    let meta = ObjectMetaData { world_transform };
    triangulate_fill(shape, style, |v| {
      self.recording.triangulated.push(v);
    });
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

fn triangulate_stroke(
  shape: &Shape,
  style: &StrokeStyle,
  vertex_visitor: impl FnMut(GraphicsVertex),
) {
  todo!()
}

fn triangulate_fill(shape: &Shape, style: &FillStyle, vertex_visitor: impl FnMut(GraphicsVertex)) {
  todo!()
}

fn visit_normalized_path(shape: &Shape, path_visitor: impl FnMut(Path2dSegment<f32>)) {
  todo!();
}
