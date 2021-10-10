use crate::*;
use rendiation_webgpu::GPU;
use std::rc::Rc;
use winit::event::Event;

#[derive(Default)]
pub struct GPUCanvas {
  current_render_buffer_size: (u32, u32),
  content: Option<Rc<wgpu::TextureView>>,
  layout: LayoutUnit,
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

pub trait CanvasPrinter {
  fn event(&mut self, event: &winit::event::Event<()>);
  fn update_render_size(&mut self, layout_size: (f32, f32), gpu: &GPU) -> (u32, u32);
  fn draw_canvas(&mut self, gpu: &GPU, canvas: Rc<wgpu::TextureView>);
}

impl<T: CanvasPrinter> Component<T> for GPUCanvas {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    model.event(event.event);
    match event.event {
      Event::MainEventsCleared => {
        let new_size = model.update_render_size(self.layout.size.into(), &event.gpu);
        if new_size != self.current_render_buffer_size {
          self.content = None;
        }

        if new_size.0 == 0 || new_size.1 == 0 {
          return;
        }

        let target = self.content.get_or_insert_with(|| {
          let device = &event.gpu.device;
          let tex = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
              width: new_size.0,
              height: new_size.1,
              depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
          });
          let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
          Rc::new(view)
        });
        model.draw_canvas(&event.gpu, target.clone());
      }
      _ => {}
    }
  }
}
