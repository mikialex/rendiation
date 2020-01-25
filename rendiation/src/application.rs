use crate::renderer::*;
use crate::window::*;
use winit::event::WindowEvent;

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
  use std::mem::size_of;
  use std::slice::from_raw_parts;

  unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

struct TestApp {
  counter: usize,
}

pub trait Application: 'static + Sized {
  fn init(renderer: &mut WGPURenderer) -> Self;
  fn update(&mut self, event: winit::event::Event<()>, renderer: &mut WGPURenderer);
  fn render(&mut self, renderer: &mut WGPURenderer);
}

pub fn run<E: Application>(title: &str) {
  use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
  };

  let event_loop = EventLoop::new();
  log::info!("Initializing the window...");

  #[cfg(not(feature = "gl"))]
  let (_window, hidpi_factor, size, surface) = {
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title(title);
    let hidpi_factor = window.hidpi_factor();
    let size = window.inner_size().to_physical(hidpi_factor);
    let surface = wgpu::Surface::create(&window);
    (window, hidpi_factor, size, surface)
  };

  #[cfg(feature = "gl")]
  let (_window, instance, hidpi_factor, size, surface) = {
    let wb = winit::WindowBuilder::new();
    let cb = wgpu::glutin::ContextBuilder::new().with_vsync(true);
    let context = cb.build_windowed(wb, &event_loop).unwrap();
    context.window().set_title(title);

    let hidpi_factor = context.window().hidpi_factor();
    let size = context
      .window()
      .get_inner_size()
      .unwrap()
      .to_physical(hidpi_factor);

    let (context, window) = unsafe { context.make_current().unwrap().split() };

    let instance = wgpu::Instance::new(context);
    let surface = instance.get_surface();

    (window, instance, hidpi_factor, size, surface)
  };

  let mut window_state: Window<()> = Window::new(
    (size.width.round() as f32, size.height.round() as f32),
    hidpi_factor as f32,
  );
  let mut renderer = WGPURenderer::new(
    surface,
    (size.width.round() as usize, size.height.round() as usize),
  );

  log::info!("Initializing the example...");
  let mut example = E::init(&mut renderer);

  log::info!("Entering render loop...");
  event_loop.run(move |event, _, control_flow| {
    // window_state.event(event.clone());
    let event_clone = event.clone();
    match event {
      event::Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        let physical = size.to_physical(hidpi_factor);
        log::info!("Resizing to {:?}", physical);
        renderer.resize((
          physical.width.round() as usize,
          physical.height.round() as usize,
        ));
      }
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested => {
          *control_flow = ControlFlow::Exit;
        }
        _ => { }
      },
      event::Event::EventsCleared => {
        example.render(&mut renderer);
      }
      _ => (),
    }
    example.update(event_clone, &mut renderer);
  });
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
