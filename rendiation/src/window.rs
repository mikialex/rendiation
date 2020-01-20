use winit::event;
use winit::event::{DeviceEvent, WindowEvent};

pub struct Window {
  size: (f32, f32),
  physical_size: (f32, f32),
  hidpi_factor: f32,
  mouse_position: (f32, f32),
}

impl Window {
  pub fn new(size: (f32, f32), hidpi_factor: f32) -> Self {
    Window {
      size,
      physical_size: (size.0 * hidpi_factor, size.1 * hidpi_factor),
      hidpi_factor,
      mouse_position: (0.0, 0.0),
    }
  }
  pub fn event(&mut self, event: winit::event::Event<()>) {
    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.size.0 = size.width as f32;
          self.size.1 = size.height as f32;
          let physical = size.to_physical(self.hidpi_factor as f64);
          self.physical_size.0 = physical.width as f32;
          self.physical_size.1 = physical.height as f32;
          log::info!("Resizing to {:?}", physical);
        }
        WindowEvent::MouseInput { button, state, .. } => {
          println!("mouse click");
        }
        WindowEvent::CursorMoved { position, .. } => {
          self.mouse_position.0 = position.x as f32;
          println!("mouse move");
        }
        _ => (),
      },
      DeviceEvent => {}
      _ => (),
    }
  }
}
