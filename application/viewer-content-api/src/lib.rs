use std::num::NonZeroIsize;

use rendiation_viewer_content::*;

pub struct ViewerAPI {
  gpu_and_surface: WGPUAndSurface,
  viewer: Viewer,
  task_spawner: TaskSpawner,
  data_source: ViewerDataScheduler,
  dyn_cx: DynCx,
}

impl ViewerAPI {
  pub fn resize(&mut self, new_width: u32, new_height: u32) {
    self
      .gpu_and_surface
      .surface
      .set_size(Size::from_u32_pair_min_one((new_width, new_height)));
  }

  pub fn create_picker_api(&mut self) -> ViewerPickerAPI {
    todo!()
  }

  pub fn render(&mut self) {
    if let Ok((canvas, target)) = self
      .gpu_and_surface
      .surface
      .get_current_frame_with_render_target_view(&self.gpu_and_surface.gpu.device)
    {
      self.viewer.draw_canvas(
        &target,
        &self.task_spawner,
        &mut self.data_source,
        &mut self.dyn_cx,
        None,
      );

      canvas.present();
    }
  }
}

pub struct ViewerPickerAPI {
  picker_impl: Box<dyn SceneModelPicker>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ViewerEntityHandle {
  pub index: u32,
  pub generation: u64,
}
impl<T> From<EntityHandle<T>> for ViewerEntityHandle {
  fn from(value: EntityHandle<T>) -> Self {
    let handle = value.into_raw();
    ViewerEntityHandle {
      index: handle.index(),
      generation: handle.generation(),
    }
  }
}
impl<T> From<ViewerEntityHandle> for EntityHandle<T> {
  fn from(value: ViewerEntityHandle) -> Self {
    unsafe { EntityHandle::from_raw(value.into()) }
  }
}
impl From<RawEntityHandle> for ViewerEntityHandle {
  fn from(value: RawEntityHandle) -> Self {
    ViewerEntityHandle {
      index: value.index(),
      generation: value.generation(),
    }
  }
}
impl From<ViewerEntityHandle> for RawEntityHandle {
  fn from(value: ViewerEntityHandle) -> Self {
    RawEntityHandle::create_only_for_testing_with_gen(value.index as usize, value.generation)
  }
}

struct NativeWin32Handle {
  hwnd: NonZeroIsize,
  size: Size,
}

impl SurfaceProvider for NativeWin32Handle {
  fn create_surface<'a>(
    &'a self,
    instance: &raw_gpu::Instance,
  ) -> Result<raw_gpu::Surface<'a>, CreateSurfaceError> {
    let window_handle = raw_gpu::rwh::Win32WindowHandle::new(self.hwnd);
    // do we need GWLP_HINSTANCE?
    let window_handle = raw_gpu::rwh::RawWindowHandle::Win32(window_handle);

    let display_handle =
      raw_gpu::rwh::RawDisplayHandle::Windows(raw_gpu::rwh::WindowsDisplayHandle::new());
    unsafe {
      instance.create_surface_unsafe(raw_gpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: display_handle,
        raw_window_handle: window_handle,
      })
    }
  }
  fn size(&self) -> Size {
    self.size
  }
}

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance(hwnd: i32) -> *mut ViewerAPI {
  let init_config = ViewerInitConfig::default();
  let gpu_platform_config = init_config.make_gpu_platform_config();

  let init_size = Size::from_u32_pair_min_one((256, 256));
  let surface = NativeWin32Handle {
    hwnd: NonZeroIsize::new(hwnd as isize).unwrap(),
    size: init_size,
  };

  let gpu_config = gpu_platform_config.make_gpu_create_config(Some((&surface, init_size)));

  let fut = WGPUAndSurface::new(gpu_config);
  let gpu_and_surface = pollster::block_on(fut);

  let worker = TaskSpawner::new("viewer-api", None);

  let viewer = Viewer::new(
    gpu_and_surface.gpu.clone(),
    gpu_and_surface.surface.clone(),
    &init_config,
    worker.clone(),
    |writer| {
      let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((1, 1)), |_, _| {
        Vec4::new(0., 0., 0., 1.)
      });
      writer.cube_texture_writer().write_cube_tex(
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
      )
    },
  );

  let api = ViewerAPI {
    gpu_and_surface,
    viewer,
    task_spawner: worker,
    data_source: Default::default(),
    dyn_cx: Default::default(),
  };
  let api = Box::new(api);
  Box::leak(api)
}

#[no_mangle]
pub extern "C" fn drop_viewer_content_api_instance(api: *mut ViewerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn viewer_resize(api: *mut ViewerAPI, new_width: u32, new_height: u32) {
  let api = unsafe { &mut *api };
  api.resize(new_width, new_height);
}

#[no_mangle]
pub extern "C" fn viewer_create_node() -> ViewerEntityHandle {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn viewer_delete_node(node: ViewerEntityHandle) {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .delete_entity(node.into());
}

#[no_mangle]
pub extern "C" fn viewer_node_attach_parent(
  node: ViewerEntityHandle,
  parent: *mut ViewerEntityHandle,
) {
  let mut writer = global_entity_component_of::<SceneNodeParentIdx, _>(|c| c.write());

  if parent.is_null() {
    writer.write(node.into(), None);
    return;
  } else {
    let parent = unsafe { *parent };
    writer.write(node.into(), Some(parent.into()));
  }
}

#[no_mangle]
pub extern "C" fn viewer_render(api: *mut ViewerAPI) {
  let api = unsafe { &mut *api };
  api.render();
}

#[no_mangle]
pub extern "C" fn viewer_create_picker_api(api: *mut ViewerAPI) -> *mut ViewerPickerAPI {
  todo!()
}

/// picker api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_picker_api(api: *mut ViewerPickerAPI) {
  todo!()
}

#[no_mangle]
pub extern "C" fn picker_pick_nearest(api: *mut ViewerPickerAPI, x: f32, y: f32) {
  todo!()
}
