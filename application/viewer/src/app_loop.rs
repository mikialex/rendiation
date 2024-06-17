use crate::*;

pub async fn run_application<T: Widget>(mut app: T) {
  let event_loop = EventLoop::new().unwrap();
  let mut window = WindowBuilder::new().build(&event_loop).unwrap();
  window.set_title("viewer");

  let minimal_required_features = rendiation_webgpu::Features::all_webgpu_mask();
  // minimal_required_features.insert(Features::TEXTURE_BINDING_ARRAY);
  // minimal_required_features.insert(Features::BUFFER_BINDING_ARRAY);
  // minimal_required_features.insert(Features::PARTIALLY_BOUND_BINDING_ARRAY);

  let config = GPUCreateConfig {
    surface_for_compatible_check_init: Some((&window, Size::from_usize_pair_min_one((300, 200)))),
    minimal_required_features,
    ..Default::default()
  };

  let (gpu, surface) = GPU::new(config).await.unwrap();
  let mut gpu = Arc::new(gpu);

  let mut surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };

  let mut window_state = WindowState::default();

  let mut event_state = PlatformEventInput::default();

  let _ = event_loop.run(move |event, target| {
    window_state.event(&event);

    event_state.queue_event(event.clone());

    if let Event::WindowEvent { ref event, .. } = event {
      match event {
        WindowEvent::CloseRequested => {
          target.exit();
        }
        WindowEvent::Resized(physical_size) => surface.resize(
          Size::from_u32_pair_min_one((physical_size.width, physical_size.height)),
          &gpu.device,
        ),
        WindowEvent::RedrawRequested => {
          let (output, mut canvas) = surface.get_current_frame_with_render_target_view().unwrap();

          let mut cx = DynCx::default();

          event_state.begin_frame();
          cx.scoped_cx(&mut window, |cx| {
            cx.scoped_cx(&mut event_state, |cx| {
              cx.scoped_cx(&mut gpu, |cx| {
                cx.scoped_cx(&mut canvas, |cx| {
                  app.update_state(cx);
                });
              });
            });
          });

          event_state.end_frame();
          cx.scoped_cx(&mut window, |cx| {
            cx.scoped_cx(&mut event_state, |cx| {
              cx.scoped_cx(&mut gpu, |cx| {
                cx.scoped_cx(&mut canvas, |cx| {
                  app.update_view(cx);
                });
              });
            });
          });

          output.present();
          window.request_redraw();
        }

        _ => {}
      };
    }
  });
}
