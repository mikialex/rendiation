use crate::util::*;
use crate::{
  camera_controls::{CameraController, CameraControllerType},
  vox::world::World,
};
use render_target::{ScreenRenderTarget, TargetStatesProvider};
use rendiation_math::Mat4;
use rendiation_render_entity::*;
use rendiation_scenegraph::*;
use rendiation_webgpu::renderer::SwapChain;
use rendiation_webgpu::*;
use rendium::*;

pub struct Rinecraft {
  pub window_session: WindowEventSession<RinecraftState>,
  pub state: RinecraftState,
}

pub struct RinecraftState {
  pub window_state: WindowState,

  pub world: World,
  pub scene: Scene<WGPURenderer>,
  pub scene_renderer: WebGPUBackend,

  pub screen_target: ScreenRenderTarget,

  pub camera_gpu: CameraGPU,
  pub camera_controller: CameraController<Self>,

  pub viewport: Viewport,
  pub gui: GUI,
}

impl RinecraftState {
  fn get_camera(&mut self) -> &mut PerspectiveCamera {
    self
      .scene
      .cameras
      .get_active_camera_mut::<PerspectiveCamera>()
  }
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self {
    let depth = WGPUTexture::new_as_depth(
      &renderer,
      wgpu::TextureFormat::Depth32Float,
      swap_chain.size,
    );
    let screen_target =
      ScreenRenderTarget::new(renderer.swap_chain_format, Some(depth), swap_chain.size);

    let gui = GUI::new(
      renderer,
      (swap_chain.size.0 as f32, swap_chain.size.1 as f32),
      &screen_target,
    );

    let mut scene = Scene::new();
    let mut world = World::new();

    // let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    // camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut camera = PerspectiveCamera::new();
    *camera.matrix_mut() = Mat4::translate(0., 40., 0.);
    camera.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
    let mut camera_gpu = CameraGPU::new(renderer, &camera, &mut scene);
    camera_gpu.update_all(renderer, &mut scene);

    scene.cameras.set_new_active_camera(camera);

    world.attach_scene(
      &mut scene,
      renderer,
      &camera_gpu,
      &screen_target.create_target_states(),
    );

    let viewport = Viewport::new(swap_chain.size);

    let mut window_session: WindowEventSession<RinecraftState> = WindowEventSession::new();

    window_session.active.resize.on(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state = &mut event_ctx.state;
      let size = (swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state
        .viewport
        .set_size(swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state.screen_target.resize(renderer, swap_chain.size);
      state.get_camera().resize(size);
      // state.camera_orth.resize(size);
      state.camera_gpu.mark_dirty();
      state.gui.renderer.resize(size, renderer);
    });

    // render
    window_session.active.event_cleared.on(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state = &mut event_ctx.state;
      let scene = &mut state.scene;

      let camera = scene.cameras.get_active_camera_mut::<PerspectiveCamera>();
      if state.camera_controller.update(camera) {
        state.camera_gpu.mark_dirty();
      }
      state.camera_gpu.update_all(renderer, scene);

      state.world.update(renderer, scene);

      let output = swap_chain.request_output();
      let output = state.screen_target.create_instance(&output.view);

      state.scene_renderer.render(scene, renderer, &output);

      state.gui.render(renderer);
      state.gui.renderer.update_to_screen(renderer, &output);

      renderer
        .queue
        .submit(&renderer.device, &mut renderer.encoder);
    });

    let window_state = WindowState::new((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    // Done
    let mut rinecraft = Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        world,
        scene,
        scene_renderer: WebGPUBackend::new(),
        camera_gpu,
        viewport,
        camera_controller: CameraController::new(),
        screen_target,
        gui,
      },
    };

    // rinecraft.state.window_state.attach_event(
    //   &mut rinecraft.window_session,
    //   |r|&mut r.window_state
    // );

    rinecraft
      .state
      .camera_controller
      .attach_event(&mut rinecraft.window_session, |r| {
        (&mut r.camera_controller, &r.window_state)
      });

    rinecraft.init_world();

    rinecraft
  }

  fn update(&mut self, event: &winit::event::Event<()>, renderer: &mut AppRenderCtx) {
    self.state.window_state.event(event);
    self.window_session.event(event, &mut self.state, renderer);
  }
}
