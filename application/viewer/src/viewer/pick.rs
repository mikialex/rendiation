use rendiation_gui_3d::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct ViewerPickerWithCtx {
  pub picker_impl: ViewerPicker,
  pub pointer_ctx: Option<ViewportPointerCtx>,
}

impl ViewerPickerWithCtx {
  fn create_ray_ctx(&self, world_ray: Ray3<f64>) -> Option<SceneRayQuery> {
    let ctx = self.pointer_ctx.as_ref()?;

    let mut ctx = create_ray_query_ctx_from_vpc(ctx);

    ctx.world_ray = world_ray;

    ctx.into()
  }

  pub fn pick_model_nearest_all(
    &self,
    world_ray: Ray3<f64>,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> {
    let cx = self.create_ray_ctx(world_ray)?;
    let mut iter = self
      .picker_impl
      .scene_model_iter_provider
      .create_ray_scene_model_iter(scene, &cx);
    pick_models_nearest(self.picker_impl.model_picker.as_ref(), &mut iter, &cx)
  }

  pub fn pick_models_list_all(
    &self,
    world_ray: Ray3<f64>,
    scene: EntityHandle<SceneEntity>,
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

    let mut iter = self
      .picker_impl
      .scene_model_iter_provider
      .create_ray_scene_model_iter(scene, &cx);

    pick_models_all(
      self.picker_impl.model_picker.as_ref(),
      &mut iter,
      &cx,
      &mut results,
      &mut models_results,
      &mut local_result_scratch,
    );
    (results, models_results)
  }
}

pub fn use_viewer_scene_model_picker(cx: &mut ViewerCx) -> Option<ViewerPickerWithCtx> {
  let scene_model_picker = use_viewer_scene_model_picker_impl(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.ndc().clone()))
    .use_assure_result(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let input = cx.input;
    let mouse_position = &input.window_state.mouse_position;

    let cam_trans = camera_transforms.expect_resolve_stage();

    let pointer_ctx =
      create_viewport_pointer_ctx(cx.active_surface_content, *mouse_position, &cam_trans);

    ViewerPickerWithCtx {
      picker_impl: scene_model_picker.unwrap(),
      pointer_ctx,
    }
    .into()
  } else {
    None
  }
}

// todo, remove duplication
impl Picker3d for ViewerPickerWithCtx {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
  ) -> Option<MeshBufferHitPoint<f64>> {
    self
      .picker_impl
      .model_picker
      .ray_query_nearest(model, &self.create_ray_ctx(world_ray)?)
  }

  fn pick_model_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    self.picker_impl.model_picker.ray_query_all(
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
      self.picker_impl.model_picker.as_ref(),
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
    pick_models_nearest(self.picker_impl.model_picker.as_ref(), models, &cx)
  }
}

pub fn prepare_picking_state<'a>(
  picker: &'a ViewerPickerWithCtx,
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
