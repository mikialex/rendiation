use crate::application::*;
use crate::geometry::*;
use crate::renderer::consts::OPENGL_TO_WGPU_MATRIX;
use crate::renderer::*;
use crate::shading::TestShading;
use crate::shading::TestShadingParamGroup;
use crate::util::*;
use crate::vox::chunk::Chunk;
use crate::watch::*;
use rendiation::*;
use rendiation_math::*;
use rendiation_render_entity::*;

impl GPUItem<PerspectiveCamera> for WGPUBuffer {
  fn create_gpu(item: &PerspectiveCamera, renderer: &mut WGPURenderer) -> Self {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();

    WGPUBuffer::new(
      &renderer.device,
      mx_ref,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    )
  }
  fn update_gpu(&mut self, item: &PerspectiveCamera, renderer: &mut WGPURenderer) {
    let mx_total = OPENGL_TO_WGPU_MATRIX * item.get_vp_matrix();
    let mx_ref: &[f32; 16] = mx_total.as_ref();
    self.update(&renderer.device, &mut renderer.encoder, mx_ref);
  }
}

impl GPUItem<ImageData> for WGPUTexture {
  fn create_gpu(image: &ImageData, renderer: &mut WGPURenderer) -> Self {
    WGPUTexture::new(&renderer.device, &mut renderer.encoder, image)
  }
  fn update_gpu(&mut self, item: &ImageData, renderer: &mut WGPURenderer) {
    todo!()
  }
}

pub struct Rinecraft {
  window_session: WindowEventSession<RinecraftState>,
  state: RinecraftState,
}

pub struct RinecraftState {
  window_state: WindowState,
  camera: GPUPair<PerspectiveCamera, WGPUBuffer>,
  orbit_controller: OrbitController,
  texture: GPUPair<ImageData, WGPUTexture>,
  cube: StandardGeometry,
  test_chunk: Chunk,
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

    window_session.add_mouse_motion_listener(|state: &mut RinecraftState, _| {
      if state.window_state.is_left_mouse_down {
        state.orbit_controller.rotate(Vec2::new(
          -state.window_state.mouse_motion.0,
          -state.window_state.mouse_motion.1,
        ))
      }
      if state.window_state.is_right_mouse_down {
        state.orbit_controller.pan(Vec2::new(
          -state.window_state.mouse_motion.0,
          -state.window_state.mouse_motion.1,
        ))
      }
    });
    window_session.add_mouse_wheel_listener(|state: &mut RinecraftState, _| {
      let delta = state.window_state.mouse_wheel_delta.1;
      state.orbit_controller.zoom(1.0 - delta * 0.1);
    });

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
        state.test_chunk.geometry.render(&mut pass);
      }
      renderer
        .queue
        .submit(&renderer.device, &mut renderer.encoder);
    });

    let window_state = WindowState::new(
      (renderer.size.0 as f32, renderer.size.1 as f32),
      renderer.hidpi_factor,
    );

    let test_chunk = Chunk::new(renderer, 0, 0);

    // Done
    Rinecraft {
      window_session,
      state: RinecraftState {
        window_state,
        cube,
        test_chunk,
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
