use crate::*;
use rendiation_webgpu::GPU;
use std::rc::Rc;
use winit::event::Event;

pub struct GPUCanvas {
  content: Option<Rc<wgpu::TextureView>>,
  quad_cache: Quad,
}

impl Default for GPUCanvas {
  fn default() -> Self {
    Self {
      content: None,
      quad_cache: Default::default(),
    }
  }
}

impl Presentable for GPUCanvas {
  fn render(&self, builder: &mut PresentationBuilder) {
    if let Some(content) = &self.content {
      builder.present.primitives.push(Primitive::Quad((
        self.quad_cache,
        Style::Texture(content.clone()),
      )));
    }
  }
}

impl LayoutAble for GPUCanvas {
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
    let size_computed = constraint.max();
    self.quad_cache.width = size_computed.width;
    self.quad_cache.height = size_computed.height;
    size_computed
  }

  fn set_position(&mut self, position: UIPosition) {
    self.quad_cache.x = position.x;
    self.quad_cache.y = position.y;
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
