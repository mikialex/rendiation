use crate::vox::world::World;
use crate::init::init_orbit_controller;
use crate::application::*;
use crate::geometry::*;
use crate::renderer::*;
use crate::shading::TestShading;
use crate::shading::TestShadingParamGroup;
use crate::util::*;
use crate::watch::*;
use rendiation::*;
use rendiation_math::*;
use rendiation_render_entity::*;

pub struct Rinecraft {
  window_session: WindowEventSession<RinecraftState>,
  state: RinecraftState,
}

pub struct RinecraftState {
  pub window_state: WindowState,
  camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  pub orbit_controller: OrbitController,
  texture: GPUPair<ImageData, WGPUTexture>,
  cube: StandardGeometry,
  world: World,
  shading: TestShading,
  shading_params: TestShadingParamGroup,
  depth: WGPUAttachmentTexture,
}

impl Application for Rinecraft {
  fn init(renderer: &mut WGPURenderer) -> Self {
    let shading = TestShading::new(renderer);

    // Create the vertex and index buffers
    let (vertex_data, index_data) = create_vertices();
    let cube = StandardGeometry::new(vertex_data, index_data, &renderer);

    // Create the texture
    let size = 512u32;
    let mut texture: GPUPair<ImageData, WGPUTexture> =
      GPUPair::new(create_texels(size as usize), renderer);
    let texture_view = texture.get_update_gpu(renderer).make_default_view();

    // Create other resources
    let sampler = WGPUSampler::new(&renderer.device);

    let mut camera = GPUPair::new(PerspectiveCamera::new(), renderer);
    camera.resize((renderer.size.0 as f32, renderer.size.1 as f32));
    camera.transform.matrix = Mat4::lookat_rh(
      Vec3::new(5f32, 5.0, 5.0),
      Vec3::new(0f32, 0.0, 0.0),
      Vec3::unit_y(),
    );

    let buffer = camera.get_update_gpu(renderer);
    let shading_params =
      TestShadingParamGroup::new(&renderer, &shading, &texture_view, &sampler, buffer);

    let depth = WGPUAttachmentTexture::new_as_depth(
      &renderer.device,
      wgpu::TextureFormat::Depth32Float,
      renderer.size,
    );

    let mut window_session = WindowEventSession::new();

    window_session.add_resize_listener(|state: &mut RinecraftState, renderer| {
      state.depth.resize(&renderer.device, renderer.size);
      state
        .camera
        .resize((renderer.size.0 as f32, renderer.size.1 as f32));
      state.camera.get_update_gpu(renderer);
    });

    init_orbit_controller(&mut window_session);

    // render
    window_session.add_events_clear_listener(|state, renderer| {
      state
        .orbit_controller
        .update(&mut state.camera as &mut PerspectiveCamera);
      state.camera.get_update_gpu(renderer);

      let output = renderer.swap_chain.request_output();
      {
        let mut pass = WGPURenderPass::build()
          .output_with_clear(&output.view, (0.1, 0.2, 0.3, 1.0))
          .with_depth(state.depth.get_view())
          .create(&mut renderer.encoder);

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
      (renderer.size.0 as f32, renderer.size.1 as f32),
      renderer.hidpi_factor,
    );

    // Done
    Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        cube,
        world: World::new(),
        camera,
        orbit_controller: OrbitController::new(),
        shading,
        shading_params,
        depth,
        texture,
      },
    }
  }

  fn update(&mut self, event: winit::event::Event<()>, renderer: &mut WGPURenderer) {
    self.state.window_state.event(event.clone());
    self.window_session.event(event, &mut self.state, renderer);
  }
}
