use crate::shading::BlockShading;
use crate::shading::BlockShadingParamGroup;
use crate::util::*;
use crate::vox::world::World;
use crate::watch::*;
use rendiation::renderer::SwapChain;
use rendiation::*;
use rendiation_render_entity::*;
use rendium::*;

pub struct Rinecraft {
  pub window_session: WindowEventSession<RinecraftState>,
  pub state: RinecraftState,
}

pub struct RinecraftState {
  pub window_state: WindowState,
  pub camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  pub camera_orth: GPUPair<ViewFrustumOrthographicCamera, WGPUBuffer>,
  pub orbit_controller: OrbitController,
  pub fps_controller: FPSController,
  pub controller_listener_handle: Vec<usize>,
  pub viewport: Viewport,
  pub cube: StandardGeometry,
  pub world: World,
  pub shading: BlockShading,
  pub shading_params: WGPUBindGroup,
  pub depth: WGPUTexture,
  pub gui: GUI,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer, swap_chain: &SwapChain) -> Self {
    let gui = GUI::new(
      renderer,
      (swap_chain.size.0 as f32, swap_chain.size.1 as f32),
    );

    let mut world = World::new();
    let block_atlas = world.world_machine.get_block_atlas(renderer);

    let depth = WGPUTexture::new_as_depth(
      &renderer,
      wgpu::TextureFormat::Depth32Float,
      swap_chain.size,
    );

    let shading = BlockShading::new(renderer, &depth);

    // Create the vertex and index buffers
    let mut cube = StandardGeometry::from(create_vertices());
    cube.update_gpu(renderer);

    // Create other resources
    let sampler = WGPUSampler::new(&renderer.device);

    let mut camera = GPUPair::new(PerspectiveCamera::new(), renderer);
    camera.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let buffer = camera.get_update_gpu(renderer);
    let shading_params = BlockShadingParamGroup {
      texture_view: &block_atlas.view(),
      sampler: &sampler,
      buffer,
    }
    .create_bindgroup(renderer);

    let viewport = Viewport::new(swap_chain.size);

    let mut window_session = WindowEventSession::new();

    window_session.add_resize_listener(|state: &mut RinecraftState, renderer| {
      let swap_chain = &mut renderer.swap_chain;
      let renderer = &mut renderer.renderer;
      let size = (swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state
        .viewport
        .set_size(swap_chain.size.0 as f32, swap_chain.size.1 as f32);
      state.depth.resize(&renderer.device, swap_chain.size);
      state.camera.resize(size);
      state.camera_orth.resize(size);
      state.camera.get_update_gpu(renderer);
      state.camera_orth.get_update_gpu(renderer);
      state.gui.renderer.resize(size, renderer);
    });

    // render
    window_session.add_events_clear_listener(|state, renderer| {
      let swap_chain = &mut renderer.swap_chain;
      let renderer = &mut renderer.renderer;
      state
        .orbit_controller
        .update(&mut state.camera_orth as &mut ViewFrustumOrthographicCamera);
      state.camera_orth.get_update_gpu(renderer);

      state
        .orbit_controller
        .update(&mut state.camera as &mut PerspectiveCamera);
      state.camera.get_update_gpu(renderer);

      state
        .world
        .update(renderer, &state.camera.get_transform().matrix.position());

      let output = swap_chain.request_output();

      {
        let mut pass = WGPURenderPass::build()
          .output_with_clear(&output.view, (0.1, 0.2, 0.3, 1.0))
          .with_depth(state.depth.view())
          .create(&mut renderer.encoder);
        pass.use_viewport(&state.viewport);

        state
          .shading
          .provide_pipeline(&mut pass, &state.shading_params);
        state.cube.render(&mut pass);
        state.world.render(&mut pass);
      }

      state.gui.render(renderer);
      state.gui.renderer.update_to_screen(renderer, &output.view);

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
        cube,
        world,
        camera,
        camera_orth,
        viewport,
        orbit_controller: OrbitController::new(),
        fps_controller: FPSController::new(),
        controller_listener_handle: Vec::new(),
        shading,
        shading_params,
        depth,
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
