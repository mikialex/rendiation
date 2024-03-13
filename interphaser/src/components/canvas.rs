use std::sync::Arc;

use rendiation_texture::Size;
use winit::event::Event;

use crate::*;

pub struct GPUCanvas {
  current_render_buffer_size: Size,
  content: Option<GPU2DTextureView>,
  drawer: Box<dyn CanvasPrinter>,
  layout: LayoutUnit,
}

impl GPUCanvas {
  pub fn new(drawer: impl CanvasPrinter + 'static) -> Self {
    Self {
      current_render_buffer_size: Size::from_u32_pair_min_one((100, 100)),
      content: None,
      drawer: Box::new(drawer),
      layout: Default::default(),
    }
  }
}

trivial_stream_impl!(GPUCanvas);
impl View for GPUCanvas {
  fn request(&mut self, detail: &mut ViewRequest) {
    match detail {
      ViewRequest::Event(ctx) => self.event(ctx),
      ViewRequest::Layout(protocol) => match protocol {
        LayoutProtocol::DoLayout {
          constraint, output, ..
        } => {
          self.layout.size = constraint.max();
          **output = self.layout.size.with_default_baseline();
        }
        LayoutProtocol::PositionAt(position) => self.layout.set_relative_position(*position),
      },
      ViewRequest::Encode(builder) => {
        self
          .layout
          .update_world(builder.current_absolution_origin());
        if let Some(content) = &self.content {
          builder.present.primitives.push(Primitive::Quad((
            self.layout.into_quad(),
            Style::Texture(content.clone()),
          )));
        }
      }
      _ => self.request(detail),
    }
  }
}

pub struct CanvasWindowPositionInfo {
  /// in window coordinates
  pub absolute_position: UIPosition,
  pub size: UISize,
}

impl CanvasWindowPositionInfo {
  pub fn compute_normalized_position_in_canvas_coordinate(
    &self,
    states: &WindowState,
  ) -> (f32, f32) {
    let canvas_x = states.mouse_position.x - self.absolute_position.x;
    let canvas_y = states.mouse_position.y - self.absolute_position.y;

    (
      canvas_x / self.size.width * 2. - 1.,
      -(canvas_y / self.size.height * 2. - 1.),
    )
  }
}

pub trait CanvasPrinter {
  fn event(
    &mut self,
    event: &winit::event::Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  );
  fn update_render_size(&mut self, layout_size: (f32, f32)) -> Size;
  fn draw_canvas(&mut self, gpu: &Arc<GPU>, canvas: GPU2DTextureView);
}

impl GPUCanvas {
  fn event(&mut self, event: &mut EventCtx) {
    let position_info = CanvasWindowPositionInfo {
      absolute_position: self.layout.absolute_position,
      size: self.layout.size,
    };

    match event.event {
      Event::WindowEvent {
        event: winit::event::WindowEvent::RedrawRequested,
        ..
      } => {
        let new_size = self.drawer.update_render_size(self.layout.size.into());
        if new_size != self.current_render_buffer_size {
          self.content = None;
          self.current_render_buffer_size = new_size;
        }

        let target = self.content.get_or_insert_with(|| {
          let device = &event.gpu.device;

          let desc = TextureDescriptor {
            label: "interphase-canvas-output".into(),
            size: map_size_gpu(new_size),
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            view_formats: &[] as &'static [rendiation_texture::TextureFormat],
            usage: TextureUsages::TEXTURE_BINDING
              | TextureUsages::COPY_DST
              | TextureUsages::COPY_SRC
              | TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
          };

          let texture = GPUTexture::create(desc, device);
          texture.create_view(Default::default()).try_into().unwrap()
        });

        self.drawer.draw_canvas(&event.gpu, target.clone());
      }
      _ => self.drawer.event(event.event, event.states, position_info),
    }
  }
}
