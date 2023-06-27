#![allow(dead_code)]
#![allow(unused)]

use crate::*;

struct PainterCtx {
  recording: GraphicsRepresentation,
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

  transform_stack: Vec<TransformState>,
  masking_stack: Vec<GraphicsRepresentation>,
  images: Vec<GraphicsImageData>,
}

impl GraphicsRepresentation {
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

  fn stroke_shape(&mut self, shape: &Shape, fill: &StrokeStyle) {
    let world_transform = self.recording.get_current_world_transform();
    let meta = ObjectMetaData { world_transform };

    todo!()
  }

  fn fill_shape(&mut self, shape: &Shape, fill: &FillStyle) {
    let world_transform = self.recording.get_current_world_transform();
    let meta = ObjectMetaData { world_transform };
    todo!()
  }

  fn push_transform(&mut self, transform: Mat3<f32>) {
    todo!()
  }

  fn pop_transform(&self) -> Mat3<f32> {
    todo!()
  }

  fn push_mask(&mut self, mask: Self::Baked) {
    self.recording.masking_stack.push(mask)
  }

  fn pop_mask(&mut self) -> Option<Self::Baked> {
    self.recording.masking_stack.pop()
  }

  fn push_filter(&mut self, effect: CanvasEffect) {}

  fn pop_filter(&mut self) -> CanvasEffect {
    todo!()
  }
}
