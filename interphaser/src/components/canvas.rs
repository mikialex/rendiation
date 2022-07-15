use crate::*;
use rendiation_texture::Size;
use std::rc::Rc;
use webgpu::{GPUTexture2d, GPUTexture2dView, WebGPUTexture2dDescriptor, GPU};
use winit::event::Event;

pub struct GPUCanvas {
  current_render_buffer_size: Size,
  content: Option<GPUTexture2dView>,
  layout: LayoutUnit,
}

impl Default for GPUCanvas {
  fn default() -> Self {
    Self {
      current_render_buffer_size: Size::from_u32_pair_min_one((100, 100)),
      content: None,
      layout: Default::default(),
    }
  }
}

impl Presentable for GPUCanvas {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset());
    if let Some(content) = &self.content {
      builder.present.primitives.push(Primitive::Quad((
        self.layout.into_quad(),
        Style::Texture(content.clone()),
      )));
    }
  }
}

impl LayoutAble for GPUCanvas {
  fn layout(&mut self, constraint: LayoutConstraint, _ctx: &mut LayoutCtx) -> LayoutResult {
    self.layout.size = constraint.max();
    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position)
  }
}

pub struct CanvasWindowPositionInfo {
  /// in window coordinates
  pub absolute_position: UIPosition,
  pub size: UISize,
}

pub trait CanvasPrinter {
  fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  );
  fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size;
  fn draw_canvas(&mut self, gpu: &Rc<GPU>, canvas: GPUTexture2dView);
}

impl<T: CanvasPrinter> Component<T> for GPUCanvas {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    let position_info = CanvasWindowPositionInfo {
      absolute_position: self.layout.absolute_position,
      size: self.layout.size,
    };

    model.event(event.event, event.states, position_info);
    match event.event {
      Event::RedrawRequested(_) => {
        let new_size = model.update_render_size(self.layout.size.into());
        if new_size != self.current_render_buffer_size {
          self.content = None;
        }

        let format = webgpu::TextureFormat::Rgba8UnormSrgb;

        let target = self.content.get_or_insert_with(|| {
          let device = &event.gpu.device;
          let texture = GPUTexture2d::create(
            WebGPUTexture2dDescriptor::from_size(new_size)
              .with_render_target_ability()
              .with_format(format),
            device,
          );
          texture.create_view(())
        });

        model.draw_canvas(&event.gpu, target.clone());
      }
      _ => {}
    }
  }
}
