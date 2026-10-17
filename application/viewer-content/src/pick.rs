use std::ops::ControlFlow;

use crate::*;

pub struct ViewerPicker {
  pub model_picker: SceneModelPickerWithViewDep<Box<dyn SceneModelPicker>>,
  pub candidate_provider: Box<dyn SceneModelCandidateProvider>,
  pub scene_bvh: Option<SceneBVHResultView>,
  pub camera_transforms: BoxedDynQuery<RawEntityHandle, CameraTransform>,
  pub clip_filter: ArrayClipPickFilter,
  pub ndc: ViewerNDC,
}

impl ViewerPicker {
  pub fn debug_bvh(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Vec<Vec<(Vec3<f32>, Vec3<f32>)>>> {
    let bvh_view = self.scene_bvh.as_ref()?;
    bvh_view.bvh.generate_bvh_debug_wireframe(scene.into_raw())
  }

  /// ray query the whole scene, hits removed by the scene's clipping planes are skipped if
  /// `remove_clipped` is true
  pub fn ray_query_scene(
    &self,
    scene: EntityHandle<SceneEntity>,
    query: &SceneRayQuery,
    remove_clipped: bool,
    v: &mut dyn RayHitVisitor,
  ) {
    self.ray_query_with_candidates(
      self.candidate_provider.as_ref(),
      scene,
      query,
      remove_clipped,
      v,
    );
  }

  /// ray query the given models, see [Self::ray_query_scene]
  pub fn ray_query_models(
    &self,
    models: &[EntityHandle<SceneModelEntity>],
    scene: EntityHandle<SceneEntity>,
    query: &SceneRayQuery,
    remove_clipped: bool,
    v: &mut dyn RayHitVisitor,
  ) {
    let candidates = ModelListCandidates(models);
    self.ray_query_with_candidates(&candidates, scene, query, remove_clipped, v);
  }

  fn ray_query_with_candidates(
    &self,
    candidates: &dyn SceneModelCandidateProvider,
    scene: EntityHandle<SceneEntity>,
    query: &SceneRayQuery,
    remove_clipped: bool,
    v: &mut dyn RayHitVisitor,
  ) {
    if remove_clipped {
      let clip_filter = self.clip_filter.create_filter(scene);
      let mut v = FilteredRayHits {
        inner: v,
        filter: |hit: &SceneRayHit| clip_filter(hit.position, hit.model),
      };
      let _ = query::ray_query_scene(&self.model_picker, candidates, scene, query, &mut v);
    } else {
      let _ = query::ray_query_scene(&self.model_picker, candidates, scene, query, v);
    }
  }

  pub fn range_pick_models(
    &self,
    scene: EntityHandle<SceneEntity>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    results: &mut Vec<EntityHandle<SceneModelEntity>>,
  ) {
    query::range_pick_models(
      &self.model_picker,
      self.candidate_provider.as_ref(),
      scene,
      frustum,
      policy,
      results,
    );
  }
}

pub struct NaiveSceneModelCandidateProvider {
  pub scene_ref_scene_model: RevRefForeignKeyRead,
}

impl NaiveSceneModelCandidateProvider {
  fn for_each_model(
    &self,
    scene: EntityHandle<SceneEntity>,
    mut f: impl FnMut(EntityHandle<SceneModelEntity>) -> ControlFlow<()>,
  ) -> ControlFlow<()> {
    if let Some(iter) = self.scene_ref_scene_model.access_multi(&scene.into_raw()) {
      for model in iter {
        f(unsafe { EntityHandle::from_raw(model) })?;
      }
    }
    ControlFlow::Continue(())
  }
}

impl SceneModelCandidateProvider for NaiveSceneModelCandidateProvider {
  fn traverse_ray(
    &self,
    scene: EntityHandle<SceneEntity>,
    _query: &SceneRayQuery,
    _t_max: f64,
    f: &mut dyn FnMut(RayCandidate) -> ControlFlow<(), f64>,
  ) -> ControlFlow<()> {
    self.for_each_model(scene, |model| {
      f(RayCandidate { model, t_enter: 0. })?;
      ControlFlow::Continue(())
    })
  }

  fn traverse_frustum(
    &self,
    scene: EntityHandle<SceneEntity>,
    _query: &SceneFrustumQuery,
    f: &mut dyn FnMut(FrustumCandidate) -> ControlFlow<()>,
  ) -> ControlFlow<()> {
    self.for_each_model(scene, |model| {
      f(FrustumCandidate {
        model,
        known_inside: false,
      })
    })
  }
}

pub struct ViewerQbvhShared(pub Arc<RwLock<FontSystem>>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for ViewerQbvhShared {
  type Result = SceneBVHResultView;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let (sm_world_bounding_valid, sm_w) = cx
      .use_shared_dual_query(SceneModelWorldBounding(self.0.clone()))
      .fork();

    let wide_line_margin = cx.use_dual_query::<WideLineWidth>().fanout(
      cx.use_db_rev_ref_tri_view::<SceneModelWideLineRenderPayload>(),
      cx,
    );
    let wide_points_margin = cx
      .use_dual_query::<WideStyledPointsMeshBuffer>()
      .use_dual_query_execute_map(cx, || {
        |_, buffer| {
          let mut max_width = 0.;
          let buffer: &[WideStyledPointVertex] = cast_slice(&buffer);
          for v in buffer {
            max_width = max_width.max(v.width);
          }
          max_width
        }
      })
      .fanout(
        cx.use_db_rev_ref_tri_view::<SceneModelWideStyledPointsRenderPayload>(),
        cx,
      );

    let together = wide_line_margin
      .dual_query_select(wide_points_margin)
      .dual_query_boxed();

    let margin = sm_w.dual_query_union(together, |(scope, margin)| match (scope, margin) {
      (Some(_), None) => Some(0.),
      (Some(_), Some(v)) => Some(v),
      _ => None,
    });

    rendiation_dynamic_bvh_scene::use_scene_dynamic_bvh(cx, sm_world_bounding_valid, margin)
  }
}

pub fn use_viewer_scene_model_picker_impl<Cx: DBHookCxLike>(
  cx: &mut Cx,
  font_system: Arc<RwLock<FontSystem>>,
  ndc: ViewerNDC,
  viewports_map: ViewportsImmediate,
  use_scene_bvh: bool,
) -> Option<ViewerPicker> {
  cx.next_scope_index();
  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let attribute_mesh_picker = use_attribute_mesh_picker(cx);
  let wide_line_picker = use_wide_line_picker(cx);
  let wide_point_picker = use_wide_points_picker(cx);
  let text_picker = use_text_picker(cx, &font_system);
  let cell_mesh_picker = use_cell_mesh_picker(cx);

  let sm_local_bounding = cx
    .use_shared_dual_query_view(SceneModelLocalBounding(font_system.clone()))
    .use_assure_result(cx);

  let sm_world_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding(font_system.clone()))
    .use_assure_result(cx);

  let qbvh = if use_scene_bvh {
    cx.scope(|cx| {
      cx.use_shared_compute(ViewerQbvhShared(font_system.clone()))
        .into_resolve_stage()
    })
  } else {
    None
  };

  let sms = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  let view_maps = cx
    .use_shared_dual_query_view(SceneModelViewDependentTransformOccShare(ndc, viewports_map))
    .use_assure_result(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(ndc))
    .use_assure_result(cx);

  let clip_filter = use_array_clip_pick_filter(cx);

  cx.when_resolve_stage(|| {
    let attribute_mesh_picker = attribute_mesh_picker.unwrap();
    let wide_line_picker = wide_line_picker.unwrap();
    let wide_point_picker = wide_point_picker.unwrap();
    let text_picker = text_picker.unwrap();
    let cell_mesh_picker = cell_mesh_picker.unwrap();

    let local_model_pickers: Vec<Box<dyn LocalModelPickerResolver>> = vec![
      Box::new(attribute_mesh_picker),
      Box::new(wide_line_picker),
      Box::new(wide_point_picker),
      Box::new(text_picker),
      Box::new(cell_mesh_picker),
    ];

    let util = SceneModelPickerBaseImplUtil {
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
      sm_world_bounding: sm_world_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      sm_local_bounding: sm_local_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
    };

    let scene_model_picker = SceneModelPickerBaseImpl {
      internal: local_model_pickers,
      util: util.clone(),
      scratch: Default::default(),
    };

    let scene_model_picker = TransformInstancedMeshPicker {
      internal: scene_model_picker,
      util,
      instance_model: read_global_db_foreign_key(),
      source_model: read_global_db_foreign_key(),
      per_unit_transform: read_global_db_component(),
      transform_buffer: read_global_db_component(),
    };

    let scene_model_picker = SceneModelPickerWithViewDep {
      internal: Box::new(scene_model_picker) as Box<dyn SceneModelPicker>,
      view_mats: view_maps.expect_resolve_stage(),
      active_view: None,
    };

    let mut scene_bvh = None;

    let candidate_provider = if let Some(qbvh) = qbvh {
      scene_bvh = Some(qbvh.clone());
      let provider =
        rendiation_dynamic_bvh_scene::SceneDynamicBvhCandidateProvider { internal: qbvh };
      Box::new(provider) as Box<dyn SceneModelCandidateProvider>
    } else {
      let provider = NaiveSceneModelCandidateProvider {
        scene_ref_scene_model: sms.expect_resolve_stage(),
      };
      Box::new(provider)
    };

    ViewerPicker {
      model_picker: scene_model_picker,
      candidate_provider,
      camera_transforms: camera_transforms.expect_resolve_stage(),
      scene_bvh,
      ndc,
      clip_filter: clip_filter.unwrap(),
    }
  })
}

pub fn create_viewport_pointer_ctx(
  surface_content: &ViewerSurfaceContent,
  mouse_position_relative_to_surface_origin: (f32, f32),
  camera_transforms: &dyn DynQuery<Key = RawEntityHandle, Value = CameraTransform>,
) -> Option<(ViewportPointerCtx, EntityHandle<SceneEntity>)> {
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

  (
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
    },
    viewport.scene,
  )
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

pub fn create_ray_query_ctx_from_vpc(
  ctx: &ViewportPointerCtx,
  extra_screen_space_tolerance: f32,
) -> SceneRayQuery {
  SceneRayQuery {
    world_ray: ctx.world_ray,
    camera_ctx: create_camera_query_ctx_from_vpc(ctx),
    extra_screen_space_tolerance,
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
    camera_max_scale: ctx.camera_world_mat.max_scale() as f32,
  }
}

pub fn create_range_pick_frustum(
  a: Vec2<f32>,
  b: Vec2<f32>,
  surface_content: &ViewerSurfaceContent,
  picker: &ViewerPicker,
  precise_intersection_test: bool,
  extra_screen_space_tolerance: f32,
) -> Option<(SceneFrustumQuery, EntityHandle<SceneEntity>)> {
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
    -1.0,
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

  let (ctx, _) =
    create_viewport_pointer_ctx(surface_content, raw_a.into(), &picker.camera_transforms)?;
  let camera_ctx = create_camera_query_ctx_from_vpc(&ctx);

  let world_helper = if precise_intersection_test {
    FrustumIntersectionTestHelper::new(&frustum)
  } else {
    None
  };

  (
    SceneFrustumQuery {
      world_frustum: frustum,
      world_helper,
      camera_ctx,
      extra_screen_space_tolerance,
    },
    viewport.scene,
  )
    .into()
}
