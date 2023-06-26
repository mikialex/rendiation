#![allow(dead_code)]
#![allow(unused)]

use crate::*;

struct PainterCtx {
  recording: GraphicsRepresentation,
}

#[derive(Default)]
struct GraphicsRepresentation {
  transform_stack: Vec<Mat3<f32>>,
  masking_stack: Vec<GraphicsRepresentation>,
  images: Vec<GraphicsImageData>,
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
    todo!()
  }

  fn fill_shape(&mut self, shape: &Shape, fill: &FillStyle) {
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
