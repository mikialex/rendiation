use core::slice;
use std::{ffi::c_void, num::NonZeroIsize, sync::Arc};

use fast_hash_collection::FastHashMap;
use rendiation_viewer_content::*;

mod cx;
use cx::*;
mod panic_hook;
pub use panic_hook::setup_panic_message_file_writer;

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
  pub fn create_view(&mut self, hwnd: *mut c_void, hinstance: *mut c_void) -> u32 {
    let init_size = Size::from_u32_pair_min_one((256, 256));

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
    let next_id = self.next_new_surface_id;
    self.next_new_surface_id += 1;

    self.surfaces.insert(next_id, surface);

    next_id
  }

  pub fn drop_surface(&mut self, id: u32) {
    self.surfaces.remove(&id);
    self.viewer.drop_surface(id);
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

  // pub fn set_device_pixel_ratio(&mut self, surface_id: u32, device_pixel_ratio: f32) {
  //  self.viewer.
  // }

  /// the size is physical resolution
  pub fn resize(&mut self, surface_id: u32, new_width: u32, new_height: u32) {
    if let Some(surface) = self.surfaces.get_mut(&surface_id) {
      surface.set_size(Size::from_u32_pair_min_one((new_width, new_height)));
    }
  }

  pub fn create_picker_api(&mut self, surface_id: u32) -> ViewerPickerAPI {
    self.viewer_api_picker_scope(|cx| {
      let picker_impl = use_viewer_scene_model_picker_impl(cx);
      let sms = cx
        .use_db_rev_ref::<SceneModelBelongsToScene>()
        .use_assure_result(cx);

      let camera_transforms = cx
        .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.ndc().clone()))
        .use_assure_result(cx);

      cx.when_resolve_stage(|| {
        let sms = sms.expect_resolve_stage();
        ViewerPickerAPI {
          picker_impl: picker_impl.unwrap(),
          camera_transforms: camera_transforms.expect_resolve_stage(),
          scene_models_of_scene: sms,
          surface_id,
        }
      })
    })
  }

  pub fn render_all_views(&mut self) {
    for (surface_id, surface) in self.surfaces.iter_mut() {
      if let Ok((canvas, target)) =
        surface.get_current_frame_with_render_target_view(&self.gpu_and_main_surface.gpu.device)
      {
        self.viewer.draw_canvas(
          *surface_id,
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
    f(&mut cx).unwrap()
  }
}

pub struct ViewerPickerAPI {
  picker_impl: Box<dyn SceneModelPicker>,
  camera_transforms: BoxedDynQuery<RawEntityHandle, CameraTransform>,
  scene_models_of_scene: RevRefForeignKeyRead,
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
    let ctx = create_viewport_pointer_ctx(
      viewer,
      self.surface_id,
      (x, y),
      todo!(),
      &self.camera_transforms,
    );

    if let Some(ctx) = ctx {
      let cx = create_ray_query_ctx_from_vpc(&ctx);

      if let Some(iter) = self.scene_models_of_scene.access_multi(&scene) {
        let iter = iter.map(|v| unsafe { EntityHandle::from_raw(v) });
        pick_models_all(
          self.picker_impl.as_ref(),
          &mut iter,
          &cx,
          &mut results,
          &mut model_results,
          &mut local_result_scratch,
        );
      }
    }

    for (r, mr) in results.iter().zip(model_results.iter()) {
      output_results.push(ViewerRayPickResult {
        primitive_index: r.primitive_index as u32,
        hit_position: r.hit.position.into_f32().into(),
        scene_model_handle: (*mr).into(),
      })
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ViewerEntityHandle {
  pub index: u32,
  pub generation: u64,
}

impl ViewerEntityHandle {
  pub fn empty() -> Self {
    Self {
      index: u32::MAX,
      generation: u64::MAX,
    }
  }
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

/// hinstance can be null_ptr
#[no_mangle]
pub extern "C" fn viewer_create_surface(
  api: *mut ViewerAPI,
  hwnd: *mut c_void,
  hinstance: *mut c_void,
) -> u32 {
  let api = unsafe { &mut *api };
  api.create_view(hwnd, hinstance)
}

#[no_mangle]
pub extern "C" fn viewer_drop_surface(api: *mut ViewerAPI, surface_id: u32) {
  let api = unsafe { &mut *api };
  api.drop_surface(surface_id)
}

/// the size is physical resolution
#[no_mangle]
pub extern "C" fn viewer_resize(
  api: *mut ViewerAPI,
  surface_id: u32,
  new_width: u32,
  new_height: u32,
) {
  let api = unsafe { &mut *api };
  api.resize(surface_id, new_width, new_height);
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
pub extern "C" fn node_set_local_mat(node: ViewerEntityHandle, mat4: *const [f64; 16]) {
  let mat4 = unsafe { *mat4 };
  let mat4 = Mat4::from(mat4);
  let mut writer = global_entity_component_of::<SceneNodeLocalMatrixComponent, _>(|c| c.write());
  writer.write(node.into(), mat4);
}

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
  normal_raw: *const f32,
  uv_raw: *const f32,
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

  let has_normal = !normal_raw.is_null();
  if has_normal {
    let normal = unsafe { slice::from_raw_parts(normal_raw, vertex_length as usize * 3) };
    let normal: &[u8] = bytemuck::cast_slice(normal);
    let normal = normal.to_vec();
    attributes.push((AttributeSemantic::Normals, normal));
  }

  let has_uv = !uv_raw.is_null();
  if has_uv {
    let uv = unsafe { slice::from_raw_parts(uv_raw, vertex_length as usize * 2) };
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

  // it's not good
  let (normal, uv) = match (has_normal, has_uv) {
    (true, true) => (
      VertexPair::from_typed(mesh.vertices[1]),
      VertexPair::from_typed(mesh.vertices[2]),
    ),
    (true, false) => (
      VertexPair::from_typed(mesh.vertices[1]),
      VertexPair::empty(),
    ),
    (false, true) => (
      VertexPair::empty(),
      VertexPair::from_typed(mesh.vertices[1]),
    ),
    (false, false) => (VertexPair::empty(), VertexPair::empty()),
  };

  AttributesMeshEntitiesCommon {
    mesh: mesh.mesh.into(),
    index: mesh.index.unwrap().into(),
    position: VertexPair::from_typed(mesh.vertices[0]),
    normal,
    uv,
    has_normal,
    has_uv,
  }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VertexPair {
  h1: ViewerEntityHandle,
  h2: ViewerEntityHandle,
}

impl VertexPair {
  fn empty() -> Self {
    Self {
      h1: ViewerEntityHandle::empty(),
      h2: ViewerEntityHandle::empty(),
    }
  }
  fn from_typed(
    handle: (
      EntityHandle<AttributesMeshEntityVertexBufferRelation>,
      EntityHandle<BufferEntity>,
    ),
  ) -> Self {
    VertexPair {
      h1: handle.0.into(),
      h2: handle.1.into(),
    }
  }
  fn into_typed(
    self,
  ) -> (
    EntityHandle<AttributesMeshEntityVertexBufferRelation>,
    EntityHandle<BufferEntity>,
  ) {
    (self.h1.into(), self.h2.into())
  }
}

#[repr(C)]
pub struct AttributesMeshEntitiesCommon {
  mesh: ViewerEntityHandle,
  index: ViewerEntityHandle,
  position: VertexPair,
  normal: VertexPair,
  has_normal: bool,
  uv: VertexPair,
  has_uv: bool,
}

#[no_mangle]
pub extern "C" fn drop_mesh(entities: AttributesMeshEntitiesCommon) {
  let mut writer = AttributesMeshEntityFromAttributesMeshWriter::from_global();
  let mut buffer = global_entity_of::<BufferEntity>().entity_writer();

  let mut vertices = Vec::new();

  vertices.push(entities.position.into_typed());
  if entities.has_normal {
    vertices.push(entities.normal.into_typed());
  }
  if entities.has_uv {
    vertices.push(entities.uv.into_typed());
  }

  let entities: AttributesMeshEntities = AttributesMeshEntities {
    mesh: entities.mesh.into(),
    index: Some(entities.index.into()),
    vertices: vertices.into(),
  };
  entities.clean_up(&mut writer, &mut buffer);
}

/// the content format expects Rgba8UnormSrgb
#[no_mangle]
pub extern "C" fn create_texture2d(
  content: *const u8,
  len: usize,
  width: u32,
  height: u32,
) -> ViewerEntityHandle {
  let data = unsafe { slice::from_raw_parts(content, len) };
  let data = data.to_vec();
  let data = GPUBufferImage {
    data,
    format: raw_gpu::TextureFormat::Rgba8UnormSrgb,
    size: Size::from_u32_pair_min_one((width, height)),
  };
  let data = MaybeUriData::Living(Arc::new(data));
  let data = ExternalRefPtr::new(data);
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<SceneTexture2dEntityDirectContent>(&Some(data)))
    .into()
}

#[no_mangle]
pub extern "C" fn drop_texture2d(handle: ViewerEntityHandle) {
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_sampler() -> ViewerEntityHandle {
  global_entity_of::<SceneSamplerEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_sampler(handle: ViewerEntityHandle) {
  global_entity_of::<SceneSamplerEntity>()
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
pub extern "C" fn unlit_material_set_color(mat: ViewerEntityHandle, color: &[f32; 4]) {
  write_global_db_component::<UnlitMaterialColorComponent>().write(mat.into(), (*color).into());
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
pub extern "C" fn pbr_mr_material_set_color(mat: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<PbrMRMaterialBaseColorComponent>().write(mat.into(), (*color).into());
}
#[no_mangle]
pub extern "C" fn pbr_mr_material_set_color_tex(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_tex_sampler::<PbrMRMaterialBaseColorAlphaTex>(mat, tex, sampler)
}

fn write_tex_sampler<C: TextureWithSamplingForeignKeys>(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_global_db_component::<SceneTexture2dRefOf<C>>().write(mat.into(), Some(tex.into()));
  write_global_db_component::<SceneSamplerRefOf<C>>().write(mat.into(), Some(sampler.into()));
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
    .new_entity(|w| w.write::<SceneCameraNode>(&Some(node.into())))
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
pub extern "C" fn viewer_render_all_views(api: *mut ViewerAPI) {
  let api = unsafe { &mut *api };
  api.render_all_views();
}

#[no_mangle]
pub extern "C" fn viewer_create_picker_api(
  api: *mut ViewerAPI,
  surface_id: u32,
) -> *mut ViewerPickerAPI {
  let api = unsafe { &mut *api };
  let api = api.create_picker_api(surface_id);
  let api = Box::new(api);
  Box::leak(api)
}

/// picker api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_picker_api(api: *mut ViewerPickerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

/// the returned pick list's should be dropped by  [drop_pick_list_result] after read the result
#[no_mangle]
pub extern "C" fn picker_pick_list(
  api: *mut ViewerPickerAPI,
  viewer: *mut ViewerAPI,
  scene: ViewerEntityHandle,
  x: f32,
  y: f32,
) -> *mut ViewerRayPickListResult {
  let api = unsafe { &mut *api };
  let viewer = unsafe { &mut *viewer };
  let mut pick_results = Vec::new();
  api.pick_list(&viewer.viewer, scene.into(), x, y, &mut pick_results);

  let r = Box::new(ViewerRayPickListResult { pick_results });
  Box::leak(r)
}

#[no_mangle]
pub extern "C" fn drop_pick_list_result(r: *mut ViewerRayPickListResult) {
  unsafe {
    let _ = Box::from_raw(r);
  };
}

pub struct ViewerRayPickListResult {
  pick_results: Vec<ViewerRayPickResult>,
}

#[repr(C)]
pub struct ViewerRayPickListResultInfo {
  pub len: usize,
  pub ptr: *const ViewerRayPickResult,
}

#[no_mangle]
pub extern "C" fn get_ray_pick_list_info(
  r: *mut ViewerRayPickListResult,
) -> ViewerRayPickListResultInfo {
  let r = unsafe { &*r };
  ViewerRayPickListResultInfo {
    len: r.pick_results.len(),
    ptr: r.pick_results.as_ptr(),
  }
}
