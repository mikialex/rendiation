use std::{num::NonZeroIsize, sync::Arc};

use fast_hash_collection::FastHashMap;
use rendiation_viewer_content::*;

mod cx;
use cx::*;

pub struct ViewerAPI {
  gpu_and_main_surface: WGPUAndSurface,
  /// the api supports multiple surface, the main surfaces also stored(cloned) here
  surfaces: FastHashMap<u32, SurfaceWrapper>,
  next_new_surface_id: u32,
  viewer: Viewer,
  picker_mem: FunctionMemory,
  task_spawner: TaskSpawner,
  data_source: ViewerDataScheduler,
  dyn_cx: DynCx,
}

impl Drop for ViewerAPI {
  fn drop(&mut self) {
    drop_viewer_from_dyn_cx(&mut self.viewer, &mut self.dyn_cx);
    self
      .picker_mem
      .cleanup(&mut ViewerAPICxDropCx as *mut ViewerAPICxDropCx as *mut ());
  }
}

impl ViewerAPI {
  // todo, we should use i32??
  pub fn create_view(&mut self, hwnd: u32) -> u32 {
    let init_size = Size::from_u32_pair_min_one((256, 256));

    let window_handle =
      raw_gpu::rwh::Win32WindowHandle::new(NonZeroIsize::new(hwnd as isize).unwrap());
    // do we need GWLP_HINSTANCE?
    let window_handle = raw_gpu::rwh::RawWindowHandle::Win32(window_handle);

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
    let next_id = self.next_new_surface_id;
    self.next_new_surface_id += 1;

    self.surfaces.insert(next_id, surface);

    next_id
  }

  pub fn drop_view(&mut self, id: u32) {
    self.surfaces.remove(&id);
  }

  pub fn new() -> Self {
    let init_config = ViewerInitConfig::default();
    let gpu_platform_config = init_config.make_gpu_platform_config();

    let gpu_config = gpu_platform_config.make_gpu_create_config(None);

    let fut = WGPUAndSurface::new_without_init_surface(gpu_config);
    let gpu_and_surface = pollster::block_on(fut);

    let worker = TaskSpawner::new("viewer-api", None);

    let viewer = Viewer::new(
      gpu_and_surface.gpu.clone(),
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

  pub fn resize(&mut self, view_id: u32, new_width: u32, new_height: u32) {
    if let Some(surface) = self.surfaces.get_mut(&view_id) {
      surface.set_size(Size::from_u32_pair_min_one((new_width, new_height)));
    }
  }

  pub fn create_picker_api(&mut self) -> ViewerPickerAPI {
    self.viewer_api_picker_scope(|cx| {
      let picker_impl = use_viewer_scene_model_picker_impl(cx);
      let sms = cx
        .use_db_rev_ref::<SceneModelBelongsToScene>()
        .use_assure_result(cx);

      cx.when_resolve_stage(|| {
        let sms = sms.expect_resolve_stage();
        ViewerPickerAPI {
          picker_impl: picker_impl.unwrap(),
          scene_models_of_scene: sms,
        }
      })
    })
  }

  pub fn render(&mut self) {
    for surface in self.surfaces.values_mut() {
      if let Ok((canvas, target)) =
        surface.get_current_frame_with_render_target_view(&self.gpu_and_main_surface.gpu.device)
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

  pub fn viewer_api_picker_scope<T>(&mut self, f: impl Fn(&mut ViewerAPICx) -> Option<T>) -> T {
    let mut pool = AsyncTaskPool::default();
    let mut immediate_results = FastHashMap::default();
    let mut change_collector = ChangeCollector::default();

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
          change_collector: &mut change_collector,
        },
        shared_ctx: &mut self.viewer.shared_ctx,
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
      shared_ctx: &mut self.viewer.shared_ctx,
      waker: futures::task::noop_waker(),
    };
    f(&mut cx).unwrap()
  }
}

pub struct ViewerPickerAPI {
  picker_impl: Box<dyn SceneModelPicker>,
  scene_models_of_scene: RevRefForeignKeyRead,
}

impl ViewerPickerAPI {
  pub fn pick_nearest(&mut self, x: f32, y: f32) {
    todo!()
  }
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

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance() -> *mut ViewerAPI {
  let api = ViewerAPI::new();
  let api = Box::new(api);
  Box::leak(api)
}

#[no_mangle]
pub extern "C" fn drop_viewer_content_api_instance(api: *mut ViewerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn viewer_create_view(api: *mut ViewerAPI, hwnd: u32) -> u32 {
  let api = unsafe { &mut *api };
  api.create_view(hwnd)
}

#[no_mangle]
pub extern "C" fn viewer_drop_view(api: *mut ViewerAPI, view_id: u32) {
  let api = unsafe { &mut *api };
  api.drop_view(view_id)
}

#[no_mangle]
pub extern "C" fn viewer_resize(
  api: *mut ViewerAPI,
  view_id: u32,
  new_width: u32,
  new_height: u32,
) {
  let api = unsafe { &mut *api };
  api.resize(view_id, new_width, new_height);
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
  let api = unsafe { &mut *api };
  let api = api.create_picker_api();
  let api = Box::new(api);
  Box::leak(api)
}

/// picker api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_picker_api(api: *mut ViewerPickerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn picker_pick_nearest(api: *mut ViewerPickerAPI, x: f32, y: f32) {
  let api = unsafe { &mut *api };
  api.pick_nearest(x, y);
}
