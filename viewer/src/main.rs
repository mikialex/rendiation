use std::time::{Duration, Instant};

use winit::{
  event::{self, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
};

fn main() {
  let event_loop = EventLoop::new();
  let mut builder = winit::window::WindowBuilder::new();
  builder = builder.with_title("viewer");
  let window = builder.build(&event_loop).unwrap();

  let viewer = futures::executor::block_on(Viewer::new(window));
  viewer.run(event_loop);
}

pub struct Viewer {
  window: winit::window::Window,
  renderer: Renderer,
  last_update_inst: Instant,
}

impl Viewer {
  pub async fn new(window: winit::window::Window) -> Self {
    let renderer = Renderer::new(&window).await;

    Self {
      window,
      renderer,
      last_update_inst: Instant::now(),
    }
  }

  pub fn run(mut self, event_loop: EventLoop<()>) {
    event_loop.run(move |event, _, control_flow| {
      *control_flow = ControlFlow::Poll;
      match event {
        event::Event::MainEventsCleared => {
          // Clamp to some max framerate to avoid busy-looping too much
          // (we might be in wgpu::PresentMode::Mailbox, thus discarding superfluous frames)
          //
          // winit has window.current_monitor().video_modes() but that is a list of all full screen video modes.
          // So without extra dependencies it's a bit tricky to get the max refresh rate we can run the window on.
          // Therefore we just go with 60fps - sorry 120hz+ folks!
          let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
          let time_since_last_frame = self.last_update_inst.elapsed();
          if time_since_last_frame >= target_frametime {
            self.window.request_redraw();
            self.last_update_inst = Instant::now();
          } else {
            *control_flow =
              ControlFlow::WaitUntil(Instant::now() + target_frametime - time_since_last_frame);
          }
        }
        event::Event::WindowEvent {
          event: WindowEvent::Resized(size),
          ..
        } => self
          .renderer
          .resize((size.width as usize, size.height as usize)),
        event::Event::WindowEvent { event, .. } => match event {
          WindowEvent::CloseRequested => {
            *control_flow = ControlFlow::Exit;
          }
          _ => {
            // example.update(event);
          }
        },
        event::Event::RedrawRequested(_) => {
          // let frame = match swap_chain.get_current_frame() {
          //   Ok(frame) => frame,
          //   Err(_) => {
          //     swap_chain = device.create_swap_chain(&surface, &sc_desc);
          //     swap_chain
          //       .get_current_frame()
          //       .expect("Failed to acquire next swap chain texture!")
          //   }
          // };

          // example.render(&frame.output, &device, &queue, &spawner);
        }
        _ => {}
      }
    });
  }
}
