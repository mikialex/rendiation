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

    let pixels_per_unit_calc = if let Some(proj_source) = ctx.proj_source {
      match proj_source {
        CommonProjection::Perspective(p) => {
          Box::new(move |d, h| p.pixels_per_unit(d, h)) as Box<dyn Fn(f32, f32) -> f32>
        }
        CommonProjection::Orth(p) => Box::new(move |d, h| p.pixels_per_unit(d, h)),
      }
    } else {
      let projection = ctx.projection;
      let projection_inv = ctx.projection_inv;
      Box::new(move |d, h| projection.pixels_per_unit(projection_inv, d, h))
    };

    SceneRayQuery {
      world_ray,
      camera_view_size_in_logic_pixel: Size::from_u32_pair_min_one(
        ctx.view_logical_pixel_size.into(),
      ),
      pixels_per_unit_calc,
      camera_world: ctx.camera_world_mat,
    }
    .into()
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

    let viewports = cx.viewer.content.viewports.iter();
    let pointer_ctx =
      if let Some((viewport, normalized_position_ndc)) = find_top_hit(viewports, *mouse_position) {
        let normalized_position_ndc: Vec2<f32> = normalized_position_ndc.into();
        let normalized_position_ndc_f64 = normalized_position_ndc.into_f64();

        let cam_trans = camera_transforms
          .expect_resolve_stage()
          .access(&viewport.camera.into_raw())
          .unwrap();
        let camera_view_projection_inv = cam_trans.view_projection_inv;
        let camera_world = cam_trans.world;

        let camera_proj = read_common_proj_from_db(viewport.camera).unwrap();

        let current_mouse_ray_in_world =
          cast_world_ray(camera_view_projection_inv, normalized_position_ndc_f64);

        let viewport_idx = cx
          .viewer
          .content
          .viewports
          .iter()
          .position(|v| v.id == viewport.id)
          .unwrap();

        let projection = camera_proj.compute_projection_mat(&OpenGLxNDC);
        let projection_inv = projection.inverse_or_identity();

        ViewportPointerCtx {
          world_ray: current_mouse_ray_in_world,
          viewport_idx,
          viewport_id: viewport.id,
          view_logical_pixel_size: view_logic_pixel_size.into_u32().into(),
          normalized_position: normalized_position_ndc,
          projection,
          projection_inv,
          proj_source: Some(camera_proj),
          camera_world_mat: camera_world,
        }
        .into()
      } else {
        None
      };

    ViewerSceneModelPicker {
      scene_model_picker: scene_model_picker.unwrap(),
      pointer_ctx,
    }
    .into()
  } else {
    None
  }
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
