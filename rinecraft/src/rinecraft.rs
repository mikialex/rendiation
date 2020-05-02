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
  pub camera_wrap: CameraGPUWrap,
  // pub camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  // pub camera_orth: GPUPair<ViewFrustumOrthographicCamera, WGPUBuffer>,
  pub orbit_controller: OrbitController,
  pub fps_controller: FPSController,
  pub controller_listener_handle: Vec<usize>,
  pub viewport: Viewport,
  pub cube: GPUGeometry,
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
    let mut cube = GPUGeometry::from(create_vertices());
    cube.update_gpu(renderer);

    // Create other resources
    let sampler = WGPUSampler::new(&renderer.device);

    // let mut camera_orth = GPUPair::new(ViewFrustumOrthographicCamera::new(), renderer);
    // camera_orth.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));

    let mut camera = PerspectiveCamera::new();
    camera.resize((swap_chain.size.0 as f32, swap_chain.size.1 as f32));
    let mut camera_wrap = CameraGPUWrap::new(renderer, camera);

    camera_wrap.update_all(renderer);
    let shading_params = BlockShadingParamGroup {
      texture_view: &block_atlas.view(),
      sampler: &sampler,
      u_mvp_matrix: &camera_wrap.gpu_mvp_matrix,
      u_camera_world_position: &camera_wrap.gpu_camera_position,
    }
    .create_bindgroup(renderer);

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
      state.depth.resize(renderer, swap_chain.size);
      state.camera_wrap.mutate_camera().resize(size);
      // state.camera_orth.resize(size);
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

      state
        .orbit_controller
        .update(state.camera_wrap.mutate_camera());
      state.camera_wrap.update_all(renderer);

      state.world.update(
        renderer,
        &state.camera_wrap.camera().get_transform().matrix.position(),
      );

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
        camera_wrap,
        // camera,
        // camera_orth,
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
