use crate::{
  camera_controls::{CameraController, CameraControllerType},
  vox::world::World,
};
use crate::{rendering::RinecraftRenderer, util::*};
use render_target::{ScreenRenderTarget, TargetInfoProvider};
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
  pub resource: ResourceManager<WebGPU>,
  pub scene: Scene<WebGPU>,

  pub screen_target: ScreenRenderTarget,

  pub perspective_projection: PerspectiveProjection,
  pub camera: Camera,
  pub camera_gpu: CameraGPU,
  pub camera_controller: CameraController<Self>,

  pub viewport: Viewport,
  pub gui: GUI,
  pub renderer: RinecraftRenderer,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self {
    let depth = WGPUTexture::new_as_depth(
      &renderer,
      rendiation_webgpu::wgpu::TextureFormat::Depth32Float,
      swap_chain.size,
    );
    let screen_target =
      ScreenRenderTarget::new(renderer.swap_chain_format, Some(depth), swap_chain.size);

    let gui = GUI::new(
      renderer,
      (swap_chain.size.0 as f32, swap_chain.size.1 as f32),
      &screen_target,
    );

    let mut resource = ResourceManager::new();
    let mut scene = Scene::new(&mut resource);
    let mut world = World::new();

    // let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    // camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut perspective_projection = PerspectiveProjection::default();
    let mut camera = Camera::new();
    *camera.matrix_mut() = Mat4::translate(0., 40., 0.);

    perspective_projection.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
    camera.update_by(&perspective_projection);

    let mut camera_gpu = CameraGPU::new(renderer, &camera, &mut resource);
    camera_gpu.update_all(&camera, renderer, &mut resource);

    world.attach_scene(
      &mut scene,
      &mut resource,
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

      state
        .perspective_projection
        .resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
      state.camera.update_by(&state.perspective_projection);

      // state.camera_orth.resize(size);
      state.gui.renderer.resize(size, renderer);
    });

    window_session.active.event_cleared.on(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state: &mut RinecraftState = &mut event_ctx.state;
      let scene = &mut state.scene;
      let resource = &mut state.resource;

      // update
      if state.camera_controller.update(&mut state.camera) {}
      state
        .camera_gpu
        .update_all(&state.camera, renderer, resource);
      state.world.update(renderer, scene, resource, &state.camera);

      let output = swap_chain.get_current_frame();
      let output = state.screen_target.create_instance(&output.view);

      // rendering
      state.renderer.render(renderer, scene, resource, &output);
      state.gui.render(renderer, &output);
    });

    let window_state = WindowState::new((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    // Done
    let mut rinecraft = Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        world,
        scene,
        resource,
        perspective_projection,
        camera,
        camera_gpu,
        viewport,
        camera_controller: CameraController::new(),
        screen_target,
        gui,
        renderer: RinecraftRenderer::new(),
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
