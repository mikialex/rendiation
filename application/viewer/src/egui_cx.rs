use egui::epaint::Shadow;
use egui::Visuals;
use rendiation_texture_gpu_process::copy_frame;
use winit::window::Window;

use crate::*;

pub struct EguiContext {
  context: egui::Context,
  state: Option<egui_winit::State>,
  renderer: Option<(egui_wgpu::Renderer, TextureFormat)>,
}

pub fn use_egui_cx(cx: &mut ApplicationCx, f: impl FnOnce(&mut ApplicationCx)) {
  let (cx, egui_cx) = cx.use_plain_state::<EguiContext>();

  if cx.processing_event {
    if egui_cx.context.is_pointer_over_area() {
      cx.dyn_cx.message.put(CameraControlBlocked);
      cx.dyn_cx.message.put(PickSceneBlocked);
    }

    access_cx!(cx.dyn_cx, window, Window);
    access_cx!(cx.dyn_cx, platform_event, PlatformEventInput);

    let state = egui_cx.state.get_or_insert_with(|| {
      let id = egui_cx.context.viewport_id();
      egui_winit::State::new(egui_cx.context.clone(), id, &window, None, None, None)
    });

    for event in &platform_event.accumulate_events {
      if let Event::WindowEvent { event, .. } = event {
        let _ = state.on_window_event(window, event);
      }
    }
  } else {
    access_cx!(cx.dyn_cx, window, Window);
    egui_cx.begin_frame(window);
  }

  // inject_cx(cx, egui_cx, |cx| f(cx)); todo
  f(cx);

  if !cx.processing_event {
    access_cx!(cx.dyn_cx, window, Window);
    access_cx!(cx.dyn_cx, gpu_and_surface, WGPUAndSurface);
    access_cx!(cx.dyn_cx, canvas, RenderTargetView);
    egui_cx.end_frame_and_draw(&gpu_and_surface.gpu, window, canvas);
  }
}

impl Default for EguiContext {
  fn default() -> EguiContext {
    let egui_context = egui::Context::default();

    const BORDER_RADIUS: u8 = 2;

    let visuals = Visuals {
      window_corner_radius: egui::CornerRadius::same(BORDER_RADIUS),
      window_shadow: Shadow::NONE,
      ..Default::default()
    };

    egui_context.set_visuals(visuals);

    EguiContext {
      context: egui_context,
      state: None,
      renderer: None,
    }
  }
}

impl EguiContext {
  pub fn begin_frame(&mut self, window: &Window) {
    let state = self.state.as_mut().unwrap();
    self.context.begin_pass(state.take_egui_input(window));
  }

  pub fn end_frame_and_draw(&mut self, gpu: &GPU, window: &Window, target: &RenderTargetView) {
    let state = self.state.as_mut().unwrap();

    let full_output = self.context.end_pass();

    state.handle_platform_output(window, full_output.platform_output);

    let tris = self
      .context
      .tessellate(full_output.shapes, full_output.pixels_per_point);

    let (renderer, fmt) = self.renderer.get_or_insert_with(|| {
      let output_color_format = target.format();
      let output_depth_format = None;
      let msaa_samples = 1;
      let renderer = egui_wgpu::Renderer::new(
        &gpu.device,
        output_color_format,
        output_depth_format,
        msaa_samples,
        false,
      );
      (renderer, output_color_format)
    });

    for (id, image_delta) in &full_output.textures_delta.set {
      renderer.update_texture(&gpu.device, &gpu.queue, *id, image_delta);
    }

    // we're not using the window size because it's may reach zero when resizing, the canvas size
    // has fixed this issue.
    let (width, height) = target.size().into_u32();
    let screen_descriptor = egui_wgpu::ScreenDescriptor {
      size_in_pixels: [width, height],
      pixels_per_point: window.scale_factor() as f32,
    };

    let mut encoder = gpu.create_encoder();

    renderer.update_buffers(
      &gpu.device,
      &gpu.queue,
      &mut encoder,
      &tris,
      &screen_descriptor,
    );

    // egui renderer only support srgb target. this is bad
    let w_target = if *fmt == target.format() {
      target.clone()
    } else {
      // in other case , we do a custom copy.
      let mut key = target.create_attachment_key();
      key.format = *fmt;
      let tex = key.create_directly(gpu);
      RenderTargetView::Texture(tex)
    };

    let mut rpass = encoder.begin_render_pass(
      RenderPassDescription::default()
        .with_name("egui main render pass")
        .with_color(&w_target, load()),
      None,
    );
    renderer.render(&mut rpass.pass, &tris, &screen_descriptor);
    drop(rpass);

    if *fmt != target.format() {
      pass("egui extra copy")
        .with_color(target, load())
        .render(&mut encoder, gpu, None)
        .by(&mut copy_frame(w_target, Some(BlendState::ALPHA_BLENDING)));
    }

    gpu.submit_encoder(encoder);

    for x in &full_output.textures_delta.free {
      renderer.free_texture(x)
    }
  }
}
