use core::slice;
use std::{num::NonZeroIsize, sync::Arc};

use fast_hash_collection::FastHashMap;
use rendiation_viewer_content::*;

mod cx;
use cx::*;

pub struct ViewerAPI {
  gpu_and_main_surface: WGPUAndInitSurface,
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

    let (gpu, _) = pollster::block_on(GPU::new(gpu_config)).unwrap();
    let gpu_and_surface = WGPUAndInitSurface { gpu, surface: None };

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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ViewerRayPickResult {
  pub primitive_index: u32,
  /// in world space. the logic hit result(maybe not exactly the ray hit point if the primitive is line or points)
  pub hit_position: [f32; 3],
  pub scene_model_handle: ViewerEntityHandle,
}

impl ViewerPickerAPI {
  pub fn pick_list(
    &mut self,
    scene: RawEntityHandle,
    x: f32,
    y: f32,
    results: &mut ViewerRayPickResult,
  ) {
    let mut results = Vec::new();
    let mut model_results = Vec::new();
    let mut local_result_scratch = Vec::new();

    let cx = todo!();

    if let Some(iter) = self.scene_models_of_scene.access_multi(&scene) {
      let iter = iter.map(|v| unsafe { EntityHandle::from_raw(v) });
      pick_models_all(
        self.picker_impl.as_ref(),
        &mut iter,
        cx,
        &mut results,
        &mut model_results,
        &mut local_result_scratch,
      );
    }

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
pub extern "C" fn create_node() -> ViewerEntityHandle {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn delete_node(node: ViewerEntityHandle) {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .delete_entity(node.into());
}

#[no_mangle]
pub extern "C" fn node_set_local_mat(node: ViewerEntityHandle, mat4: *const [f32; 16]) {}

#[no_mangle]
pub extern "C" fn node_attach_parent(node: ViewerEntityHandle, parent: *mut ViewerEntityHandle) {
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
pub extern "C" fn create_mesh(
  indices_length: u32,
  indices: *const u32,
  vertex_length: u32,
  position: *const f32,
  normal: *const f32,
  uv: *const f32,
  topo: MeshPrimitiveTopology,
) -> AttributesMeshEntitiesCommon {
  let indices = unsafe { slice::from_raw_parts(indices, indices_length as usize) };
  let indices: &[u8] = bytemuck::cast_slice(indices);
  let indices = indices.to_vec();

  let mut attributes = Vec::new();

  let position = unsafe { slice::from_raw_parts(position, vertex_length as usize * 3) };
  let position: &[u8] = bytemuck::cast_slice(position);
  let position = position.to_vec();
  attributes.push((AttributeSemantic::Positions, position));

  if !normal.is_null() {
    let normal = unsafe { slice::from_raw_parts(normal, vertex_length as usize * 3) };
    let normal: &[u8] = bytemuck::cast_slice(normal);
    let normal = normal.to_vec();
    attributes.push((AttributeSemantic::Normals, normal));
  }

  if !uv.is_null() {
    let uv = unsafe { slice::from_raw_parts(uv, vertex_length as usize * 2) };
    let uv: &[u8] = bytemuck::cast_slice(uv);
    let uv = uv.to_vec();
    attributes.push((AttributeSemantic::TexCoords(0), uv));
  }

  let mut writer = AttributesMeshEntityFromAttributesMeshWriter::from_global();
  let mut buffer = global_entity_of::<BufferEntity>().entity_writer();
  let mesh = AttributesMeshData {
    attributes,
    indices: Some((AttributeIndexFormat::Uint32, indices)),
    mode: topo,
  }
  .build()
  .write(&mut writer, &mut buffer);

  AttributesMeshEntitiesCommon {
    mesh: mesh.mesh.into(),
  }
}

#[repr(C)]
pub struct AttributesMeshEntitiesCommon {
  mesh: ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn drop_mesh(handle: AttributesMeshEntitiesCommon) {
  //
}

#[no_mangle]
pub extern "C" fn create_texture2d() -> ViewerEntityHandle {
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_texture2d(handle: ViewerEntityHandle) {
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_unlit_material() -> ViewerEntityHandle {
  global_entity_of::<UnlitMaterialEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_unlit_material(handle: ViewerEntityHandle) {
  global_entity_of::<UnlitMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn create_pbr_mr_material() -> ViewerEntityHandle {
  global_entity_of::<PbrMRMaterialEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_pbr_mr_material(handle: ViewerEntityHandle) {
  global_entity_of::<PbrMRMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn create_wide_line() -> ViewerEntityHandle {
  global_entity_of::<WideLineModelEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_wide_line(handle: ViewerEntityHandle) {
  global_entity_of::<WideLineModelEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_text3d() -> ViewerEntityHandle {
  global_entity_of::<Text3dEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_text3d(handle: ViewerEntityHandle) {
  global_entity_of::<Text3dEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_camera(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<SceneCameraEntity>()
    .entity_writer()
    .new_entity(|w| {
      //
      w.write::<SceneCameraNode>(&Some(node.into()))
    })
    .into()
}
#[no_mangle]
pub extern "C" fn drop_camera(handle: ViewerEntityHandle) {
  global_entity_of::<SceneCameraEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_dir_light(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<DirectionalLightEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<DirectionalRefNode>(&Some(node.into())))
    .into()
}

#[no_mangle]
pub extern "C" fn drop_dir_light(handle: ViewerEntityHandle) {
  global_entity_of::<DirectionalLightEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

// #[no_mangle]
// pub extern "C" fn create_wide_point() -> ViewerEntityHandle {
//   // global_entity_of::<SceneModelEntity>()
//   //   .entity_writer()
//   //   .new_entity(|w| w)
//   //   .into()
//   todo!()
// }

// #[no_mangle]
// pub extern "C" fn drop_wide_point(handle: ViewerEntityHandle) {
//   todo!()
// }

#[no_mangle]
pub extern "C" fn create_scene_model(
  material: ViewerEntityHandle,
  mesh: ViewerEntityHandle,
) -> ViewerEntityHandle {
  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn drop_scene_model(handle: ViewerEntityHandle) {
  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(handle.into());
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
pub extern "C" fn picker_pick_list(
  api: *mut ViewerPickerAPI,
  scene: ViewerEntityHandle,
  x: f32,
  y: f32,
  results: &mut ViewerRayPickResult,
) {
  let api = unsafe { &mut *api };
  api.pick_list(scene.into(), x, y, results);
}
