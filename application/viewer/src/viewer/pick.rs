use crate::*;

pub struct ViewerPickerWithCtx {
  pub picker_impl: ViewerPicker,
  pub pointer_ctx: Option<(ViewportPointerCtx, EntityHandle<SceneEntity>)>,
}

impl ViewerPickerWithCtx {
  pub fn pick_range(
    &self,
    scene: EntityHandle<SceneEntity>,
    frustum: &SceneFrustumQuery,
    ty: ObjectTestPolicy,
  ) -> Vec<EntityHandle<SceneModelEntity>> {
    let mut results = Vec::default();
    self
      .picker_impl
      .range_pick_models(scene, frustum, ty, &mut results);
    results
  }

  fn create_ray_query(
    &self,
    world_ray: Ray3<f64>,
  ) -> Option<(SceneRayQuery, EntityHandle<SceneEntity>)> {
    let (pointer_ctx, scene) = self.pointer_ctx.as_ref()?;
    let mut query = create_ray_query_ctx_from_vpc(pointer_ctx, 0.);
    query.world_ray = world_ray;
    Some((query, *scene))
  }

  pub fn pick_model_nearest_all(
    &self,
    world_ray: Ray3<f64>,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<SceneRayHit> {
    let (query, _) = self.create_ray_query(world_ray)?;
    let mut v = NearestRayHit::default();
    self
      .picker_impl
      .ray_query_scene(scene, &query, true, &mut v);
    v.nearest
  }

  /// all hits sorted by distance
  pub fn pick_models_list_all(
    &self,
    world_ray: Ray3<f64>,
    scene: EntityHandle<SceneEntity>,
  ) -> Vec<SceneRayHit> {
    let Some((query, _)) = self.create_ray_query(world_ray) else {
      return Vec::new();
    };
    let mut v = AllRayHits::default();
    self
      .picker_impl
      .ray_query_scene(scene, &query, true, &mut v);
    v.into_sorted()
  }
}

pub fn use_viewer_scene_model_picker(cx: &mut ViewerCx) -> Option<ViewerPickerWithCtx> {
  let scene_model_picker = use_viewer_scene_model_picker_impl(
    cx,
    cx.viewer.font_system.clone(),
    *cx.viewer.ndc(),
    cx.viewer.viewport_map.clone(),
    cx.viewer.use_scene_bvh,
  );

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(*cx.viewer.ndc()))
    .use_assure_result(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let input = cx.input;
    let mouse_position = &input.window_state.mouse_position;

    let cam_trans = camera_transforms.expect_resolve_stage();

    let pointer_ctx =
      create_viewport_pointer_ctx(cx.active_surface_content, *mouse_position, &cam_trans);

    let mut picker_impl = scene_model_picker.unwrap();
    picker_impl
      .model_picker
      .set_active_view(pointer_ctx.as_ref().map(|c| c.0.viewport_id));

    ViewerPickerWithCtx {
      picker_impl,
      pointer_ctx,
    }
    .into()
  } else {
    None
  }
}

impl Picker3d for ViewerPickerWithCtx {
  fn ray_query_models(
    &self,
    models: &[EntityHandle<SceneModelEntity>],
    world_ray: Ray3<f64>,
    v: &mut dyn RayHitVisitor,
  ) {
    if let Some((query, scene)) = self.create_ray_query(world_ray) {
      self
        .picker_impl
        .ray_query_models(models, scene, &query, true, v);
    }
  }
}

pub fn prepare_picking_state<'a>(
  picker: &'a ViewerPickerWithCtx,
  g: &WidgetSceneModelIntersectionGroupConfig,
) -> Option<Interaction3dCtx<'a>> {
  let pointer_ctx = picker.pointer_ctx.as_ref()?;
  let group: Vec<_> = g.group.iter().copied().collect();
  let world_ray_intersected_nearest = picker.pick_models_nearest(&group, pointer_ctx.0.world_ray);

  Some(Interaction3dCtx {
    picker: picker as &dyn Picker3d,
    world_ray_intersected_nearest,
  })
}
