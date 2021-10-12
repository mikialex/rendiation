use crate::*;
use rendiation_texture::Size;
use rendiation_webgpu::{GPUTextureSize, GPU};
use std::rc::Rc;
use winit::event::Event;

pub struct GPUCanvas {
  current_render_buffer_size: Size,
  content: Option<Rc<wgpu::TextureView>>,
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
    self.layout.update_world(builder.current_origin_offset);
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

pub struct FrameTarget {
  pub size: Size,
  pub format: wgpu::TextureFormat,
  pub view: Rc<wgpu::TextureView>,
}

pub trait CanvasPrinter {
  fn event(&mut self, event: &winit::event::Event<()>);
  fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size;
  fn draw_canvas(&mut self, gpu: &Rc<GPU>, canvas: FrameTarget);
}

impl<T: CanvasPrinter> Component<T> for GPUCanvas {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    model.event(event.event);
    match event.event {
      Event::MainEventsCleared => {
        let new_size = model.update_render_size(self.layout.size.into());
        if new_size != self.current_render_buffer_size {
          self.content = None;
        }

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let target = self.content.get_or_insert_with(|| {
          let device = &event.gpu.device;
          let tex = device.create_texture(&wgpu::TextureDescriptor {
            size: new_size.into_gpu_size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
          });
          let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
          Rc::new(view)
        });

        let target = FrameTarget {
          size: new_size,
          format,
          view: target.clone(),
        };

        model.draw_canvas(&event.gpu, target);
      }
      _ => {}
    }
  }
}
