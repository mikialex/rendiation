use std::sync::{
  atomic::{AtomicI32, Ordering},
  Arc,
};

use futures::{channel::oneshot::Sender, FutureExt};
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
  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.rendering.ndc))
    .use_assure_result(cx);

  let use_attribute_mesh_picker = use_attribute_mesh_picker(cx);
  let wide_line_picker = use_wide_line_picker(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
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
    };

    let view_logic_pixel_size = Vec2::new(
      cx.input.window_state.physical_size.0 / cx.input.window_state.device_pixel_ratio,
      cx.input.window_state.physical_size.1 / cx.input.window_state.device_pixel_ratio,
    )
    .map(|v| v.ceil() as u32);
    let view_logic_pixel_size = Size::from_u32_pair_min_one(view_logic_pixel_size.into());

    let scene_model_picker: Box<dyn SceneModelPicker> = Box::new(scene_model_picker);
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
      scene_model_picker,
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

pub fn compute_normalized_position_in_canvas_coordinate(
  offset: (f32, f32),
  size: (f32, f32),
) -> (f32, f32) {
  (offset.0 / size.0 * 2. - 1., -(offset.1 / size.1 * 2. - 1.))
}

#[derive(Default)]
pub struct GPUxEntityIdMapPicker {
  last_id_buffer_size: Option<Size>,
  wait_to_read_tasks: Vec<(Sender<ReadTextureFromStagingBuffer>, ReadRange)>,
  unresolved_counter: Arc<AtomicI32>,
  // this is to trigger the render when on demand rendering is enabled
  // another way or optimization is to keep a id buffer locally for any time to pick
  waker: Option<Waker>,
}

impl GPUxEntityIdMapPicker {
  pub fn last_id_buffer_size(&self) -> Option<Size> {
    self.last_id_buffer_size
  }
  pub fn read_new_frame_id_buffer(
    &mut self,
    texture: &GPUTypedTextureView<TextureDimension2, u32>,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    waker: &Waker,
  ) {
    let full_size = texture.size();
    self.last_id_buffer_size = Some(full_size);
    for (sender, range) in self.wait_to_read_tasks.drain(..) {
      if let Some(range) = range.clamp(full_size) {
        sender
          .send(encoder.read_texture_2d(&gpu.device, texture, range))
          .ok();
      } // else the sender will drop, and receiver will resolved
    }
    self.waker = Some(waker.clone());
  }

  pub fn notify_frame_id_buffer_not_available(&mut self) {
    self.wait_to_read_tasks.clear();
    self.last_id_buffer_size = None;
  }

  pub fn pick_point_at(
    &mut self,
    pixel_position: (usize, usize),
  ) -> Option<Box<dyn Future<Output = Option<u32>> + Unpin>> {
    let range = ReadRange {
      size: Size::from_usize_pair_min_one((1, 1)),
      offset_x: pixel_position.0,
      offset_y: pixel_position.1,
    };
    let f = self.pick_ids(range)?;
    let f = f.map(|result| result.map(|ids| ids.first().copied().unwrap_or(0)));

    if let Some(w) = self.waker.take() {
      w.wake();
    }

    Some(Box::new(f))
  }

  /// resolved to None if gpu read failed or read cancelled because of the read range is out of bound.
  ///
  /// - the picking result is not deduplicated
  /// - the result id only contains entity index, without generational info, so it's possible to access
  ///   wrong or deleted entity because of the unsynced entity change happened in same entity position.
  pub fn pick_ids(
    &mut self,
    range: ReadRange,
  ) -> Option<Pin<Box<dyn Future<Output = Option<Vec<u32>>>>>> {
    if self.unresolved_counter.load(Ordering::Relaxed) > 100 {
      return None;
    }

    let counter = self.unresolved_counter.clone();
    counter.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver) = futures::channel::oneshot::channel();
    self.wait_to_read_tasks.push((sender, range));

    Some(Box::pin(
      async {
        let texture_read_future = receiver.await.ok()?;
        let texture_read_buffer = texture_read_future.await.ok()?;
        let buffer = texture_read_buffer.read_into_raw_unpadded_buffer();
        let buffer: &[u32] = bytemuck::cast_slice(&buffer); // todo fix potential alignment issue
        Some(buffer.to_vec())
      }
      .map(move |r| {
        counter.fetch_sub(1, Ordering::Relaxed);
        r
      }),
    ))
  }
}
