use crate::*;

pub struct ViewerPicker {
  pub model_picker: Box<dyn SceneModelPicker>,
  pub scene_model_iter_provider: Box<dyn SceneModelIterProvider>,
}

pub struct NaiveSceneModelIterProvider {
  pub scene_ref_scene_model: RevRefForeignKeyRead,
}

impl SceneModelIterProvider for NaiveSceneModelIterProvider {
  fn create_ray_scene_model_iter(
    &self,
    scene: EntityHandle<SceneEntity>,
    _ctx: &SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    if let Some(iter) = self.scene_ref_scene_model.access_multi(&scene.into_raw()) {
      let iter = iter.map(|v| unsafe { EntityHandle::from_raw(v) });
      Box::new(iter)
    } else {
      Box::new([].into_iter())
    }
  }
}

pub fn use_viewer_scene_model_picker_impl<Cx: DBHookCxLike>(
  cx: &mut Cx,
  font_system: Arc<RwLock<FontSystem>>,
) -> Option<ViewerPicker> {
  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let use_attribute_mesh_picker = use_attribute_mesh_picker(cx);
  let wide_line_picker = use_wide_line_picker(cx);

  let sm_local_bounding = cx
    .use_shared_dual_query_view(SceneModelLocalBounding(font_system.clone()))
    .use_assure_result(cx);

  let sm_world_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding(font_system))
    .use_assure_result(cx);

  let sms = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  cx.when_resolve_stage(|| {
    let att_mesh_picker = use_attribute_mesh_picker.unwrap();
    let wide_line_picker = wide_line_picker.unwrap();

    let local_model_pickers: Vec<Box<dyn LocalModelPicker>> =
      vec![Box::new(att_mesh_picker), Box::new(wide_line_picker)];

    let scene_model_picker = SceneModelPickerBaseImpl {
      internal: local_model_pickers,
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

    let iter_provider = NaiveSceneModelIterProvider {
      scene_ref_scene_model: sms.expect_resolve_stage(),
    };

    ViewerPicker {
      model_picker: Box::new(scene_model_picker),
      scene_model_iter_provider: Box::new(iter_provider),
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

  let view_physical_pixel_size = viewport.viewport.yw();

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
    camera_view_size_in_logic_pixel: Size::from_u32_pair_min_one(
      ctx.view_logical_pixel_size.into(),
    ),
    pixels_per_unit_calc: ctx.create_ratio_cal(),
    camera_world: ctx.camera_world_mat,
  }
}
