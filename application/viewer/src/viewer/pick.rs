use rendiation_gui_3d::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct ViewerSceneModelPicker {
  scene_model_picker: Box<dyn SceneModelPicker>,
  pub pointer_ctx: Option<ViewportPointerCtx>,
}

impl ViewerSceneModelPicker {
  fn create_ray_ctx(&self, world_ray: Ray3<f64>) -> Option<SceneRayQuery> {
    let ctx = self.pointer_ctx.as_ref()?;

    let mut ctx = create_ray_query_ctx_from_vpc(ctx);

    ctx.world_ray = world_ray;

    ctx.into()
  }
}

pub fn use_viewer_scene_model_picker(cx: &mut ViewerCx) -> Option<ViewerSceneModelPicker> {
  let scene_model_picker = use_viewer_scene_model_picker_impl(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.ndc().clone()))
    .use_assure_result(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let view_logic_pixel_size = Vec2::new(
      cx.input.window_state.physical_size.0 / cx.input.window_state.device_pixel_ratio,
      cx.input.window_state.physical_size.1 / cx.input.window_state.device_pixel_ratio,
    )
    .map(|v| v.ceil() as u32);
    let view_logic_pixel_size = Size::from_u32_pair_min_one(view_logic_pixel_size.into());

    let input = cx.input;
    let mouse_position = &input.window_state.mouse_position;

    let cam_trans = camera_transforms.expect_resolve_stage();

    let pointer_ctx = create_viewport_pointer_ctx(
      cx.viewer,
      *mouse_position,
      view_logic_pixel_size,
      &cam_trans,
    );

    ViewerSceneModelPicker {
      scene_model_picker: scene_model_picker.unwrap(),
      pointer_ctx,
    }
    .into()
  } else {
    None
  }
}

impl Picker3d for ViewerSceneModelPicker {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
  ) -> Option<MeshBufferHitPoint<f64>> {
    self
      .scene_model_picker
      .ray_query_nearest(model, &self.create_ray_ctx(world_ray)?)
  }

  fn pick_model_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    self.scene_model_picker.ray_query_all(
      idx,
      &self.create_ray_ctx(world_ray)?,
      results,
      local_result_scratch,
    )
  }

  fn pick_models_all(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3<f64>,
  ) -> (
    Vec<MeshBufferHitPoint<f64>>,
    Vec<EntityHandle<SceneModelEntity>>,
  ) {
    let cx = self.create_ray_ctx(world_ray);

    if cx.is_none() {
      return (Vec::new(), Vec::new());
    }
    let cx = cx.unwrap();

    let mut results = Vec::default();
    let mut models_results = Vec::default();
    let mut local_result_scratch = Vec::default();
    pick_models_all(
      self.scene_model_picker.as_ref(),
      models,
      &cx,
      &mut results,
      &mut models_results,
      &mut local_result_scratch,
    );
    (results, models_results)
  }

  fn pick_models_nearest(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3<f64>,
  ) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> {
    let cx = self.create_ray_ctx(world_ray)?;
    pick_models_nearest(self.scene_model_picker.as_ref(), models, &cx)
  }
}

pub fn prepare_picking_state<'a>(
  picker: &'a ViewerSceneModelPicker,
  g: &WidgetSceneModelIntersectionGroupConfig,
) -> Option<Interaction3dCtx<'a>> {
  let pointer_ctx = picker.pointer_ctx.as_ref()?;
  let world_ray_intersected_nearest =
    picker.pick_models_nearest(&mut g.group.iter().copied(), pointer_ctx.world_ray);

  Some(Interaction3dCtx {
    picker: picker as &dyn Picker3d,
    world_ray_intersected_nearest,
  })
}
