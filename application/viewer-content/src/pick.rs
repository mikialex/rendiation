use crate::*;

pub struct ViewerPicker {
  pub model_picker: SceneModelPickerWithViewDep<Box<dyn SceneModelPicker>>,
  pub scene_model_iter_provider: Box<dyn SceneModelIterProvider>,
  pub bvh: LockReadGuardHolder<rendiation_qbvh_scene::SceneQbvh>,
  pub camera_transforms: BoxedDynQuery<RawEntityHandle, CameraTransform>,
  pub ndc: ViewerNDC,
}

pub struct NaiveSceneModelIterProvider {
  pub scene_ref_scene_model: RevRefForeignKeyRead,
}

impl NaiveSceneModelIterProvider {
  fn create_full_scene_iter(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    if let Some(iter) = self.scene_ref_scene_model.access_multi(&scene.into_raw()) {
      let iter = iter.map(|v| unsafe { EntityHandle::from_raw(v) });
      Box::new(iter)
    } else {
      Box::new([].into_iter())
    }
  }
}

impl SceneModelIterProvider for NaiveSceneModelIterProvider {
  fn create_ray_scene_model_iter(
    &self,
    scene: EntityHandle<SceneEntity>,
    _ctx: &SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    self.create_full_scene_iter(scene)
  }

  fn create_frustum_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    _frustum: &SceneFrustumQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    self.create_full_scene_iter(scene)
  }
}

pub fn use_viewer_scene_model_picker_impl<Cx: DBHookCxLike>(
  cx: &mut Cx,
  font_system: Arc<RwLock<FontSystem>>,
  ndc: ViewerNDC,
  viewports_map: ViewportsImmediate,
) -> Option<ViewerPicker> {
  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let use_attribute_mesh_picker = use_attribute_mesh_picker(cx);
  let wide_line_picker = use_wide_line_picker(cx);
  let wide_point_picker = use_wide_points_picker(cx);

  let sm_local_bounding = cx
    .use_shared_dual_query_view(SceneModelLocalBounding(font_system.clone()))
    .use_assure_result(cx);

  let sm_world_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding(font_system.clone()))
    .use_assure_result(cx);

  let (sm_world_bounding_valid, sm_w) = cx
    .use_shared_dual_query(SceneModelWorldBounding(font_system))
    .dual_query_filter_map(|v| v)
    .fork();
  let margin = sm_w.dual_query_map(|_| 0.); // todo, use correct margin source
  let qbvh = rendiation_qbvh_scene::use_scene_qbvh(cx, sm_world_bounding_valid, margin);

  let sms = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  let view_maps = cx
    .use_shared_dual_query_view(SceneModelViewDependentTransformOccShare(ndc, viewports_map))
    .use_assure_result(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(ndc.clone()))
    .use_assure_result(cx);

  cx.when_resolve_stage(|| {
    let att_mesh_picker = use_attribute_mesh_picker.unwrap();
    let wide_line_picker = wide_line_picker.unwrap();
    let wide_point_picker = wide_point_picker.unwrap();

    let local_model_pickers: Vec<Box<dyn LocalModelPicker>> = vec![
      Box::new(att_mesh_picker),
      Box::new(wide_line_picker),
      Box::new(wide_point_picker),
    ];

    let scene_model_picker = SceneModelPickerBaseImpl {
      internal: local_model_pickers,
      selectable: read_global_db_component(),
      scene_model_node: read_global_db_foreign_key(),
      node_world: node_world
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      node_net_visible: node_net_visible
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      filter: Some(Box::new(create_clip_pick_filter())),
      sm_world_bounding: sm_world_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      sm_local_bounding: sm_local_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
    };

    let scene_model_picker = SceneModelPickerWithViewDep {
      internal: Box::new(scene_model_picker) as Box<dyn SceneModelPicker>,
      view_mats: view_maps.expect_resolve_stage(),
      active_view: None,
    };

    let iter_provider = NaiveSceneModelIterProvider {
      scene_ref_scene_model: sms.expect_resolve_stage(),
    };

    ViewerPicker {
      model_picker: scene_model_picker,
      scene_model_iter_provider: Box::new(iter_provider),
      camera_transforms: camera_transforms.expect_resolve_stage(),
      bvh: qbvh.unwrap(),
      ndc,
    }
  })
}

pub fn create_viewport_pointer_ctx(
  surface_content: &ViewerSurfaceContent,
  mouse_position_relative_to_surface_origin: (f32, f32),
  camera_transforms: &dyn DynQuery<Key = RawEntityHandle, Value = CameraTransform>,
) -> Option<ViewportPointerCtx> {
  let (viewport, normalized_position_ndc) = find_top_hit(
    surface_content.viewports.iter(),
    mouse_position_relative_to_surface_origin,
  )?;

  let normalized_position_ndc: Vec2<f32> = normalized_position_ndc.into();
  let normalized_position_ndc_f64 = normalized_position_ndc.into_f64();

  let cam_trans = camera_transforms
    .access(&viewport.camera.into_raw())
    .unwrap();
  let camera_view_projection_inv = cam_trans.view_projection_inv;
  let camera_world = cam_trans.world;

  let camera_proj = read_common_proj_from_db(viewport.camera).unwrap();

  let current_mouse_ray_in_world =
    cast_world_ray(camera_view_projection_inv, normalized_position_ndc_f64);

  let viewport_idx = surface_content
    .viewports
    .iter()
    .position(|v| v.id == viewport.id)
    .unwrap();

  let projection = camera_proj.compute_projection_mat(&OpenGLxNDC);
  let projection_inv = projection.inverse_or_identity();

  let view_physical_pixel_size = viewport.viewport.zw();

  let view_logical_pixel_size = Vec2::new(
    view_physical_pixel_size.x() / surface_content.device_pixel_ratio,
    view_physical_pixel_size.y() / surface_content.device_pixel_ratio,
  )
  .map(|v| v.ceil() as u32);

  let view_logical_pixel_size = Size::from_u32_pair_min_one(view_logical_pixel_size.into());
  let view_logical_pixel_size = view_logical_pixel_size.into_u32().into();

  ViewportPointerCtx {
    world_ray: current_mouse_ray_in_world,
    viewport_idx,
    viewport_id: viewport.id,
    view_logical_pixel_size,
    normalized_position: normalized_position_ndc,
    projection,
    projection_inv,
    proj_source: Some(camera_proj),
    camera_world_mat: camera_world,
  }
  .into()
}

pub fn read_common_proj_from_db(
  camera: EntityHandle<SceneCameraEntity>,
) -> Option<CommonProjection> {
  let pp = read_global_db_component::<SceneCameraPerspective>();
  let po = read_global_db_component::<SceneCameraOrthographic>();
  pp.get_value(camera)
    .flatten()
    .map(CommonProjection::Perspective)
    .or_else(|| po.get_value(camera).flatten().map(CommonProjection::Orth))
}

pub fn create_ray_query_ctx_from_vpc(ctx: &ViewportPointerCtx) -> SceneRayQuery {
  SceneRayQuery {
    world_ray: ctx.world_ray,
    camera_ctx: create_camera_query_ctx_from_vpc(ctx),
  }
}

pub fn create_camera_query_ctx_from_vpc(ctx: &ViewportPointerCtx) -> CameraQueryCtx {
  CameraQueryCtx {
    camera_view_size_in_logic_pixel: Size::from_u32_pair_min_one(
      ctx.view_logical_pixel_size.into(),
    ),
    pixels_per_unit_calc: ctx.create_ratio_cal(),
    camera_world: ctx.camera_world_mat,
    camera_vp: ctx.projection.into_f64() * ctx.camera_world_mat.inverse_or_identity(),
  }
}

pub fn create_range_pick_frustum(
  a: Vec2<f32>,
  b: Vec2<f32>,
  surface_content: &ViewerSurfaceContent,
  picker: &ViewerPicker,
) -> Option<SceneFrustumQuery> {
  let raw_a = a;
  let a = a * surface_content.device_pixel_ratio;
  let b = b * surface_content.device_pixel_ratio;

  let (viewport, normalized_a) = find_top_hit(surface_content.viewports.iter(), a.into())?;
  let (viewport_, normalized_b) = find_top_hit(surface_content.viewports.iter(), b.into())?;
  if viewport.id != viewport_.id {
    return None;
  }
  let a = Vec2::from(normalized_a);
  let b = Vec2::from(normalized_b);

  let min = a.min(b);
  let max = a.max(b);

  let ndc_arr = [
    min.x as f64,
    max.x as f64,
    min.y as f64,
    max.y as f64,
    0.0,
    1.0,
  ];

  let camera = viewport.camera;
  let camera_trans = picker
    .camera_transforms
    .access(camera.raw_handle_ref())
    .unwrap();

  let mat =
    picker.ndc.transform_into_opengl_standard_ndc().into_f64() * camera_trans.view_projection;
  let frustum = Frustum::new_from_matrix_ndc(mat, &ndc_arr);

  let ctx = create_viewport_pointer_ctx(surface_content, raw_a.into(), &picker.camera_transforms)?;
  let camera_ctx = create_camera_query_ctx_from_vpc(&ctx);

  SceneFrustumQuery {
    world_frustum: frustum,
    camera_ctx,
  }
  .into()
}
