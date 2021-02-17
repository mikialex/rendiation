use crate::{
  application::{AppRenderCtx, Application},
  camera::VoxlandCamera,
  camera_controls::{CameraController, CameraControllerType},
  vox::world::World,
  window_event::WindowEventSession,
  window_states::WindowState,
};
use crate::{rendering::VoxlandRenderer, util::*};
use render_target::{ScreenRenderTarget, TargetInfoProvider};
use rendiation_algebra::Mat4;
use rendiation_render_entity::*;
use rendiation_scenegraph::*;
use rendiation_webgpu::renderer::SwapChain;
use rendiation_webgpu::*;

pub struct Voxland {
  pub window_session: WindowEventSession<VoxlandState>,
  pub state: VoxlandState,
}

pub struct VoxlandState {
  pub window_state: WindowState,

  pub world: World,
  pub resource: ResourceManager<WebGPU>,
  pub scene: Scene<WebGPU>,

  pub screen_target: ScreenRenderTarget,

  pub camera: VoxlandCamera,
  pub camera_controller: CameraController<Self>,

  // pub gui: GUI,
  pub renderer: VoxlandRenderer,
  pub rt: tokio::runtime::Runtime,
}

impl Application for Voxland {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self {
    let depth = WGPUTexture::new_as_depth(
      &renderer,
      rendiation_webgpu::wgpu::TextureFormat::Depth32Float,
      swap_chain.size,
    );
    let screen_target =
      ScreenRenderTarget::new(renderer.swap_chain_format, Some(depth), swap_chain.size);

    // let gui = GUI::new(
    //   renderer,
    //   (swap_chain.size.0 as f32, swap_chain.size.1 as f32),
    //   &screen_target,
    // );

    let mut resource = ResourceManager::new();
    let mut scene = Scene::new(&mut resource);
    let mut world = World::new();

    // let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    // camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut perspective_projection = PerspectiveProjection::default();
    let mut camera = VoxlandCamera::new(&mut resource, swap_chain.size);
    *camera.camera_mut().matrix_mut() = Mat4::translate(0., 40., 0.);

    // let mut camera = Camera::new();
    // *camera.matrix_mut() = Mat4::translate(0., 40., 0.);

    perspective_projection.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
    camera.camera_mut().update_by(&perspective_projection);

    world.attach_scene(
      &mut scene,
      &mut resource,
      renderer,
      &camera,
      &screen_target.create_target_states(),
    );

    let mut window_session: WindowEventSession<VoxlandState> = WindowEventSession::new();

    window_session.active.resize.on(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state = &mut event_ctx.state;
      let size = (swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state.screen_target.resize(renderer, swap_chain.size);
      state.camera.resize(swap_chain.size);

      // state.gui.renderer.resize(size, renderer);
    });

    window_session.active.event_cleared.on(|event_ctx| {
      let swap_chain = &mut event_ctx.render_ctx.swap_chain;
      let renderer = &mut event_ctx.render_ctx.renderer;
      let state: &mut VoxlandState = &mut event_ctx.state;
      let scene = &mut state.scene;
      let resource = &mut state.resource;

      // update
      if state.camera_controller.update(&mut state.camera) {}
      state.camera.update(resource);
      state
        .rt
        .block_on(state.world.update(renderer, scene, resource, &state.camera));

      let output = swap_chain.get_current_frame();
      let output = state.screen_target.create_instance(&output.view);

      // rendering
      state
        .renderer
        .render(renderer, scene, &state.camera, resource, &output);
      // state.gui.render(renderer, &output);
    });

    let window_state = WindowState::new((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    // Done
    let mut voxland = Voxland {
      window_session,
      state: VoxlandState {
        window_state,
        world,
        scene,
        resource,
        camera,
        camera_controller: CameraController::new(),
        screen_target,
        // gui,
        renderer: VoxlandRenderer::new(),
        rt: tokio::runtime::Runtime::new().unwrap(),
      },
    };

    // voxland.state.window_state.attach_event(
    //   &mut voxland.window_session,
    //   |r|&mut r.window_state
    // );

    voxland
      .state
      .camera_controller
      .attach_event(&mut voxland.window_session, |r| {
        (&mut r.camera_controller, &r.window_state)
      });

    voxland.init_world();

    voxland
  }

  fn update(&mut self, event: &winit::event::Event<()>, renderer: &mut AppRenderCtx) {
    self.state.window_state.event(event);
    self.window_session.event(event, &mut self.state, renderer);
  }
}
