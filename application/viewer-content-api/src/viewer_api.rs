use crate::*;

pub struct ViewerAPI {
  gpu_and_main_surface: WGPUAndInitSurface,
  /// the api supports multiple surface, the main surfaces also stored(cloned) here
  surfaces: FastHashMap<u32, SurfaceWrapper>,
  next_new_surface_id: u32,
  pub(crate) viewer: Viewer,
  picker_mem: FunctionMemory,
  task_spawner: TaskSpawner,
  data_source: ViewerDataScheduler,
  dyn_cx: DynCx,
}

impl Drop for ViewerAPI {
  fn drop(&mut self) {
    let mut drop_cx = ViewerAPICxDropCx {
      dyn_cx: &mut self.dyn_cx,
      shared_ctx: &mut self.viewer.shared_ctx,
    };

    self
      .picker_mem
      .cleanup(&mut drop_cx as *mut ViewerAPICxDropCx as *mut ());

    drop_viewer_from_dyn_cx(&mut self.viewer, &mut self.dyn_cx);
  }
}

impl ViewerAPI {
  /// note, in surface creation logic, we create default node and scene and camera, these
  /// entity will leaked if we not handle it well, but not a big problem
  /// todo fix
  pub fn create_surface(
    &mut self,
    hwnd: *mut c_void,
    hinstance: *mut c_void,
    width: u32,
    height: u32,
  ) -> u32 {
    let init_size = Size::from_u32_pair_min_one((width, height));

    let mut window_handle =
      raw_gpu::rwh::Win32WindowHandle::new(NonZeroIsize::new(hwnd as isize).unwrap());

    if !hinstance.is_null() {
      window_handle.hinstance = Some(NonZeroIsize::new(hinstance as isize).unwrap());
    }
    let window_handle = raw_gpu::rwh::RawWindowHandle::Win32(window_handle);

    // display handle in windows is always default.
    let display_handle =
      raw_gpu::rwh::RawDisplayHandle::Windows(raw_gpu::rwh::WindowsDisplayHandle::new());
    let surface = unsafe {
      self
        .gpu_and_main_surface
        .gpu
        .instance
        .create_surface_unsafe(raw_gpu::SurfaceTargetUnsafe::RawHandle {
          raw_display_handle: display_handle,
          raw_window_handle: window_handle,
        })
    }
    .unwrap();

    let surface = GPUSurface::new(
      &self.gpu_and_main_surface.gpu.adaptor,
      &self.gpu_and_main_surface.gpu.device,
      surface,
      init_size,
    );

    // here we pray the caller not drop the window!
    let surface = SurfaceWrapper::new(surface, Arc::new(hwnd));
    let surface_id = self.next_new_surface_id;
    self.next_new_surface_id += 1;

    self.surfaces.insert(surface_id, surface);

    let widget_scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let root = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let camera_node = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity(|w| {
        w.write::<SceneNodeLocalMatrixComponent>(&Mat4::lookat(
          Vec3::new(3., 3., 3.),
          Vec3::new(0., 0., 0.),
          Vec3::new(0., 1., 0.),
        ))
      });

    let camera = global_entity_of::<SceneCameraEntity>()
      .entity_writer()
      .new_entity(|w| {
        w.write::<SceneCameraPerspective>(&Some(PerspectiveProjection::default()))
          .write::<SceneCameraBelongsToScene>(&scene.some_handle())
          .write::<SceneCameraNode>(&camera_node.some_handle())
      });

    let viewport = Vec4::new(0., 0., width as f32, height as f32);

    let viewports = vec![ViewerViewPort {
      id: alloc_global_res_id(),
      viewport,
      camera,
      camera_node,
      debug_camera_for_view_related: None,
    }];

    let background = {
      let mut writer = SceneWriter::from_global(scene);

      let default_env_background = load_example_cube_tex(&mut writer);
      ViewerBackgroundState::init(default_env_background, &mut writer)
    };

    let scene = ViewerSurfaceContent {
      scene,
      root,
      selected_dir_light: None,
      selected_model: Default::default(),
      selected_point_light: None,
      selected_spot_light: None,
      widget_scene,
      viewports,
      device_pixel_ratio: 1.0,
      background,
    };
    self.viewer.surfaces_content.insert(surface_id, scene);

    self.resize(surface_id, width, height);

    surface_id
  }

  pub fn drop_surface(&mut self, surface_id: u32) {
    self.surfaces.remove(&surface_id);
    self.viewer.drop_surface(surface_id);
  }

  pub fn read_last_render_result(&mut self, surface_id: u32) -> Option<GPUBufferImage> {
    let view = self.viewer.rendering.surface_views.get(&surface_id)?;
    let view = view.values().next()?; // we only have one view
    let result = view.direct_read_cached_frame_sync(&self.gpu_and_main_surface.gpu)?;

    let data = result.read_into_raw_unpadded_buffer();

    GPUBufferImage {
      data,
      format: result.info().format,
      size: result.info().size(),
    }
    .into()
  }

  pub fn set_surface_scene(&mut self, surface_id: u32, scene: RawEntityHandle) {
    let content = self
      .viewer
      .surfaces_content
      .get_mut(&surface_id)
      .expect("surface content missing");

    content.scene = unsafe { EntityHandle::from_raw(scene) }
  }

  pub fn set_surface_camera(&mut self, surface_id: u32, camera: RawEntityHandle) {
    let content = self
      .viewer
      .surfaces_content
      .get_mut(&surface_id)
      .expect("surface content missing");
    let camera_node = get_db_view::<SceneCameraNode>()
      .read_ref(camera)
      .unwrap()
      .unwrap();
    unsafe {
      content.viewports[0].camera = EntityHandle::from_raw(camera);
      content.viewports[0].camera_node = EntityHandle::from_raw(camera_node);
    }
  }

  pub fn new(mut init_config: ViewerInitConfig) -> Self {
    // setup some necessary config for viewer api's use case
    init_config.always_enable_caching_frame_for_direct_read = true;

    let gpu_platform_config = init_config.make_gpu_platform_config();

    let gpu_config = gpu_platform_config.make_gpu_create_config(None);

    let (gpu, _) = pollster::block_on(GPU::new(gpu_config)).unwrap();
    let gpu_and_surface = WGPUAndInitSurface { gpu, surface: None };

    let worker = TaskSpawner::new("viewer-api", None);

    let viewer = Viewer::new(gpu_and_surface.gpu.clone(), &init_config, worker.clone());

    ViewerAPI {
      gpu_and_main_surface: gpu_and_surface,
      surfaces: Default::default(),
      next_new_surface_id: 0,
      viewer,
      task_spawner: worker,
      data_source: Default::default(),
      dyn_cx: Default::default(),
      picker_mem: Default::default(),
    }
  }

  pub fn set_device_pixel_ratio(&mut self, surface_id: u32, device_pixel_ratio: f32) {
    if let Some(content) = self.viewer.surfaces_content.get_mut(&surface_id) {
      content.device_pixel_ratio = device_pixel_ratio;
    } else {
      log::warn!("unable to find surface")
    }
  }

  /// the size is physical resolution
  pub fn resize(&mut self, surface_id: u32, new_width: u32, new_height: u32) {
    if let Some(surface) = self.surfaces.get_mut(&surface_id) {
      surface.set_size(Size::from_u32_pair_min_one((new_width, new_height)));
    } else {
      log::warn!("unable to find surface")
    }

    if let Some(content) = self.viewer.surfaces_content.get_mut(&surface_id) {
      let vp = &mut content.viewports[0];
      vp.viewport.z = new_width as f32;
      vp.viewport.w = new_height as f32;
    } else {
      log::warn!("unable to find surface content")
    }
  }

  pub fn create_picker_api(&mut self, surface_id: u32) -> ViewerPickerAPI {
    self.viewer_api_picker_scope(|cx| {
      cx.viewer.update_view_ty_immediate();
      let picker_impl = use_viewer_scene_model_picker_impl(
        cx,
        cx.viewer.font_system.clone(),
        cx.viewer.ndc().clone(),
        cx.viewer.viewport_map.clone(),
        cx.viewer.use_scene_bvh,
      );

      cx.when_resolve_stage(|| {
        let active_surface = cx.viewer.surfaces_content.get(&surface_id).unwrap();
        let active_view = active_surface.viewports[0].id;
        let mut picker_impl = picker_impl.unwrap();
        picker_impl.model_picker.set_active_view(Some(active_view));

        ViewerPickerAPI {
          picker_impl,
          surface_id,
        }
      })
    })
  }

  pub fn render_surface(&mut self, surface_id: u32) {
    setup_new_frame_allocator(10 * 1024 * 1024);
    self.viewer.update_view_ty_immediate();
    if let Some(surface) = self.surfaces.get(&surface_id) {
      if let Ok((canvas, target)) =
        surface.get_current_frame_with_render_target_view(&self.gpu_and_main_surface.gpu.device)
      {
        unsafe {
          self
            .dyn_cx
            .register_cx::<ViewerDataScheduler>(&mut self.data_source);
        };

        self.viewer.draw_canvas(
          surface_id,
          &target,
          &self.task_spawner,
          &mut self.data_source,
          &mut self.dyn_cx,
          None,
        );

        unsafe {
          self.dyn_cx.unregister_cx::<ViewerDataScheduler>();
        };

        canvas.present();
      }
    }
  }

  pub fn viewer_api_picker_scope<T>(&mut self, f: impl Fn(&mut ViewerAPICx) -> Option<T>) -> T {
    let mut pool = AsyncTaskPool::default();
    let mut immediate_results = FastHashMap::default();

    unsafe {
      self
        .dyn_cx
        .register_cx::<ViewerDataScheduler>(&mut self.data_source);
    };

    {
      self.viewer.shared_ctx.reset_visiting();
      immediate_results.clear();
      let mut cx = ViewerAPICx {
        memory: &mut self.picker_mem,
        dyn_cx: &mut self.dyn_cx,
        stage: ViewerAPICxStage::Spawn {
          spawner: &self.task_spawner,
          pool: &mut pool,
          immediate_results: &mut immediate_results,
        },
        viewer: &mut self.viewer,
        waker: futures::task::noop_waker(),
      };

      let r = f(&mut cx);
      assert!(r.is_none());
    }

    let mut task_pool_result = pollster::block_on(pool.all_async_task_done());

    self.viewer.shared_ctx.reset_visiting();
    task_pool_result
      .token_based_result
      .extend(immediate_results.drain());
    immediate_results.clear();

    let mut cx = ViewerAPICx {
      memory: &mut self.picker_mem,
      dyn_cx: &mut self.dyn_cx,
      stage: ViewerAPICxStage::Resolve {
        result: &mut task_pool_result,
      },
      viewer: &mut self.viewer,
      waker: futures::task::noop_waker(),
    };
    let r = f(&mut cx).unwrap();

    unsafe {
      self.dyn_cx.unregister_cx::<ViewerDataScheduler>();
    };

    r
  }
}

pub struct ViewerPickerAPI {
  picker_impl: ViewerPicker,
  surface_id: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ViewerRayPickResult {
  pub primitive_index: u32,
  /// in world space. the logic hit result(maybe not exactly the ray hit point if the primitive is line or points)
  pub hit_position: [f32; 3],
  pub scene_model_handle: ViewerEntityHandle,
}

impl ViewerPickerAPI {
  /// the x, y is logic pixel
  pub fn pick_list(
    &mut self,
    viewer: &Viewer,
    scene: RawEntityHandle,
    x: f32,
    y: f32,
    output_results: &mut Vec<ViewerRayPickResult>,
  ) {
    let mut results = Vec::new();
    let mut model_results = Vec::new();
    let mut local_result_scratch = Vec::new();
    let surface_content = viewer.surfaces_content.get(&self.surface_id).unwrap();
    let ctx =
      create_viewport_pointer_ctx(surface_content, (x, y), &self.picker_impl.camera_transforms);

    if let Some(ctx) = ctx {
      let cx = create_ray_query_ctx_from_vpc(&ctx);

      let scene = unsafe { EntityHandle::from_raw(scene) };
      let mut iter = self
        .picker_impl
        .scene_model_iter_provider
        .create_ray_scene_model_iter(scene, &cx);

      pick_models_all(
        &self.picker_impl.model_picker,
        &mut iter,
        &cx,
        &mut results,
        &mut model_results,
        &mut local_result_scratch,
      );
    }

    for (r, mr) in results.iter().zip(model_results.iter()) {
      output_results.push(ViewerRayPickResult {
        primitive_index: r.primitive_index as u32,
        hit_position: r.hit.position.into_f32().into(),
        scene_model_handle: (*mr).into(),
      })
    }
  }

  /// all inputs are logic pixel
  pub fn pick_range(
    &mut self,
    viewer: &Viewer,
    scene: RawEntityHandle,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    output_results: &mut Vec<ViewerEntityHandle>,
    contain: bool,
  ) {
    let scene = unsafe { EntityHandle::from_raw(scene) };
    let a = Vec2::new(ax, ay);
    let b = Vec2::new(bx, by);

    let surface_content = viewer.surfaces_content.get(&self.surface_id).unwrap();
    if let Some(frustum) = create_range_pick_frustum(a, b, surface_content, &self.picker_impl) {
      let mut iter = self
        .picker_impl
        .scene_model_iter_provider
        .create_frustum_scene_model_iter(scene, &frustum);

      range_pick_models(
        &self.picker_impl.model_picker,
        &mut iter,
        &frustum,
        if contain {
          ObjectTestPolicy::Contains
        } else {
          ObjectTestPolicy::Intersect
        },
        &mut |r| output_results.push(r.into_raw().into()),
      );
      //
    }
  }
}
