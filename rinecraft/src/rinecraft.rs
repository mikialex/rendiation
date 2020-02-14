use crate::application::*;
use crate::geometry::*;
use crate::init::init_orbit_controller;
use crate::renderer::*;
use crate::shading::BlockShading;
use crate::shading::BlockShadingParamGroup;
use crate::util::*;
use crate::vox::world::World;
use crate::watch::*;
use rendiation::*;
use rendiation_render_entity::*;
use image::ImageBuffer;
use image::Rgba;

pub struct Rinecraft {
  pub window_session: WindowEventSession<RinecraftState>,
  pub state: RinecraftState,
}

type ImageData = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub struct RinecraftState {
  pub window_state: WindowState,
  pub camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  pub orbit_controller: OrbitController,
  pub fps_controller: FPSController,
  pub controller_listener_handle: Vec<usize>,
  pub viewport: Viewport,
  cube: StandardGeometry,
  world: World,
  shading: BlockShading,
  shading_params: BlockShadingParamGroup,
  depth: WGPUAttachmentTexture,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer) -> Self {
    let mut world = World::new();
    let block_atlas_view = world.world_machine.create_block_atlas_gpu(renderer);

    let shading = BlockShading::new(renderer);

    // Create the vertex and index buffers
    let cube = StandardGeometry::new_pair(create_vertices(), &renderer);

    // Create other resources
    let sampler = WGPUSampler::new(&renderer.device);

    let mut camera = GPUPair::new(PerspectiveCamera::new(), renderer);
    camera.resize((renderer.size.0 as f32, renderer.size.1 as f32));

    let buffer = camera.get_update_gpu(renderer);
    let shading_params =
      BlockShadingParamGroup::new(&renderer, &shading, &block_atlas_view, &sampler, buffer);

    let depth = WGPUAttachmentTexture::new_as_depth(
      &renderer.device,
      wgpu::TextureFormat::Depth32Float,
      renderer.size,
    );

    let viewport = Viewport::new(renderer.size);

    let mut window_session = WindowEventSession::new();

    window_session.add_resize_listener(|state: &mut RinecraftState, renderer| {
      state.depth.resize(&renderer.device, renderer.size);
      state
        .camera
        .resize((renderer.size.0 as f32, renderer.size.1 as f32));
      state.camera.get_update_gpu(renderer);
    });

    init_orbit_controller(&mut window_session);

    window_session.add_mouse_down_listener(|state: &mut RinecraftState, _| {
      let x_ratio = state.window_state.mouse_position.0 / state.window_state.size.0;
      let y_ratio = 1. - state.window_state.mouse_position.1 / state.window_state.size.1;
      assert!(x_ratio <= 1.);
      assert!(y_ratio <= 1.);
      let ray = state.camera.create_screen_ray(x_ratio, y_ratio);
      state.world.delete_block_by_ray(&ray);
    });

    // render
    window_session.add_events_clear_listener(|state, renderer| {
      state
        .orbit_controller
        .update(&mut state.camera as &mut PerspectiveCamera);
      state.camera.get_update_gpu(renderer);

      state
        .world
        .update(renderer, &state.camera.get_transform().matrix.position());

      let output = renderer.swap_chain.request_output();
      {
        let mut pass = WGPURenderPass::build()
          .output_with_clear(&output.view, (0.1, 0.2, 0.3, 1.0))
          .with_depth(state.depth.get_view())
          .create(&mut renderer.encoder);
        pass.use_viewport(&state.viewport);

        state
          .shading
          .provide_pipeline(&mut pass, &state.shading_params);
        state.cube.render(&mut pass);
        state.world.render(&mut pass);
      }
      renderer
        .queue
        .submit(&renderer.device, &mut renderer.encoder);
    });

    let window_state = WindowState::new(
      (
        renderer.size.0 as f32 / renderer.hidpi_factor,
        renderer.size.1 as f32 / renderer.hidpi_factor,
      ),
      renderer.hidpi_factor,
    );

    // Done
    Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        cube,
        world,
        camera,
        viewport,
        orbit_controller: OrbitController::new(),
        fps_controller: FPSController::new(),
        controller_listener_handle: Vec::new(),
        shading,
        shading_params,
        depth,
      },
    }
  }

  fn update(&mut self, event: winit::event::Event<()>, renderer: &mut WGPURenderer) {
    self.state.window_state.event(event.clone());
    self.window_session.event(event, &mut self.state, renderer);
  }
}
