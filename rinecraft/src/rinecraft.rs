use crate::util::*;
use crate::vox::world::World;
use render_target::{RenderTargetAble, ScreenRenderTarget, ScreenRenderTargetInstance};
use rendiation::renderer::SwapChain;
use rendiation::*;
use rendiation_render_entity::*;
use rendium::*;
use scene::scene::Scene;

pub struct Rinecraft {
  pub window_session: WindowEventSession<RinecraftState>,
  pub state: RinecraftState,
}

pub struct RinecraftState {
  pub window_state: WindowState,
  pub scene: Scene,
  pub camera_gpu: CameraGPU,
  // pub camera_orth: GPUPair<ViewFrustumOrthographicCamera, WGPUBuffer>,
  pub orbit_controller: OrbitController,
  pub fps_controller: FPSController,
  pub controller_listener_handle: Vec<usize>,
  pub viewport: Viewport,
  pub world: World,
  pub screen_target: ScreenRenderTarget,
  pub gui: GUI,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self {
    let gui = GUI::new(
      renderer,
      (swap_chain.size.0 as f32, swap_chain.size.1 as f32),
    );

    let mut scene = Scene::new();
    let mut world = World::new();

    let depth = WGPUTexture::new_as_depth(
      &renderer,
      wgpu::TextureFormat::Depth32Float,
      swap_chain.size,
    );

    let screen_target = ScreenRenderTarget::new(renderer.swap_chain_format, Some(depth));

    // let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    // camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut camera = PerspectiveCamera::new();
    camera.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
    let mut camera_gpu = CameraGPU::new(renderer, &camera);
    camera_gpu.update_all(renderer, &camera);

    scene.set_new_active_camera(camera);

    world.attach_scene(
      &mut scene,
      renderer,
      &camera_gpu,
      &screen_target.create_target_states(),
    );

    let viewport = Viewport::new(swap_chain.size);

    let mut window_session: WindowEventSession<RinecraftState> = WindowEventSession::new();

    window_session.add_resize_listener(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state = &mut event_ctx.state;
      let size = (swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state
        .viewport
        .set_size(swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state.screen_target.resize(renderer, swap_chain.size);
      state
        .scene
        .get_active_camera_mut_downcast::<PerspectiveCamera>()
        .resize(size);
      // state.camera_orth.resize(size);
      state.camera_gpu.mark_dirty();
      state.gui.renderer.resize(size, renderer);
    });

    // render
    window_session.add_events_clear_listener(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state = &mut event_ctx.state;
      // state
      //   .orbit_controller
      //   .update(&mut state.camera_orth as &mut ViewFrustumOrthographicCamera);
      // state.camera_orth.get_update_gpu(renderer);

      let camera = state
        .scene
        .get_active_camera_mut_downcast::<PerspectiveCamera>();
      if state.orbit_controller.update(camera) {
        state.camera_gpu.mark_dirty();
      }
      state.camera_gpu.update_all(renderer, camera);

      state.world.update(renderer, &mut state.scene);

      state.scene.prepare(renderer);

      let output = swap_chain.request_output();
      let output = ScreenRenderTargetInstance {
        swap_chain_view: &output.view,
        base: &state.screen_target,
      };

      state.scene.render(&output, renderer);

      state.gui.render(renderer);
      state.gui.renderer.update_to_screen(renderer, &output);

      renderer
        .queue
        .submit(&renderer.device, &mut renderer.encoder);
    });

    let window_state = WindowState::new(
      (
        swap_chain.size.0 as f32 / swap_chain.hidpi_factor,
        swap_chain.size.1 as f32 / swap_chain.hidpi_factor,
      ),
      swap_chain.hidpi_factor,
    );

    // Done
    let mut rinecraft = Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        world,
        scene,
        camera_gpu,
        // camera_orth,
        viewport,
        orbit_controller: OrbitController::new(),
        fps_controller: FPSController::new(),
        controller_listener_handle: Vec::new(),
        screen_target,
        gui,
      },
    };

    rinecraft.use_orbit_controller();
    rinecraft.init_world();

    rinecraft
  }

  fn update(&mut self, event: winit::event::Event<()>, renderer: &mut AppRenderCtx) {
    self.state.window_state.event(event.clone());
    self.window_session.event(event, &mut self.state, renderer);
  }
}
