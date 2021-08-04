use crate::*;
use rendiation_webgpu::GPU;
use std::rc::Rc;
use winit::event::Event;

#[derive(Default)]
pub struct GPUCanvas {
  content: Option<Rc<wgpu::TextureView>>,
  layout: LayoutUnit,
}

impl Presentable for GPUCanvas {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    if let Some(content) = &self.content {
      builder.present.primitives.push(Primitive::Quad((
        self.layout.into_quad(),
        Style::Texture(content.clone()),
      )));
    }
  }
}

impl LayoutAble for GPUCanvas {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
    self.layout.size = constraint.max();
    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position)
  }
}

pub trait CanvasPrinter {
  fn event(&mut self, event: &winit::event::Event<()>);
  fn render_size(&self) -> (f32, f32);
  fn draw_canvas(&mut self, gpu: &GPU, canvas: &wgpu::TextureView);
}

impl<T: CanvasPrinter> Component<T> for GPUCanvas {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    model.event(event.event);
    match event.event {
      Event::MainEventsCleared => {
        let target = self.content.get_or_insert_with(|| {
          // todo diff render size change
          let device = &event.gpu.device;
          let tex = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
              width: model.render_size().0 as u32,
              height: model.render_size().1 as u32,
              depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            label: None,
          });
          let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
          Rc::new(view)
        });
        model.draw_canvas(&event.gpu, target);
      }
      _ => {}
    }
  }
}
