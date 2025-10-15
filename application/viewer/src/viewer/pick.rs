use std::sync::{
  atomic::{AtomicI32, Ordering},
  Arc,
};

use database::global_entity_component_of;
use futures::{channel::oneshot::Sender, FutureExt};
use rendiation_gui_3d::*;
use rendiation_mesh_core::MeshBufferIntersectConfig;
use rendiation_scene_geometry_query::*;
use rendiation_wide_line::*;

use crate::*;

pub struct ViewerPicker {
  current_mouse_ray_in_world: Ray3<f64>,
  normalized_position: Vec2<f32>,
  normalized_position_ndc: Vec2<f32>,
  conf: MeshBufferIntersectConfig,
  camera_view_size: Size,
  scene_model_picker: Box<dyn SceneModelPicker>,
}

pub fn use_viewer_picker(cx: &mut ViewerCx) -> Option<ViewerPicker> {
  let sm_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding)
    .use_assure_result(cx);

  let mesh_vertex_refs = cx
    .use_db_rev_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .use_assure_result(cx);

  let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
  let node_net_visible = use_global_node_net_visible_view(cx).use_assure_result(cx);

  let camera_transforms = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(cx.viewer.rendering.ndc))
    .use_assure_result(cx);

  let wide_line_sm_bounding = cx
    .use_shared_dual_query_view(WideLineWorldBounding)
    .use_assure_result(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let camera_view_projection_inv = camera_transforms
      .expect_resolve_stage()
      .access(&cx.viewer.scene.main_camera.into_raw())
      .unwrap()
      .view_projection_inv;

    let att_mesh_picker = AttributeMeshPicker {
      sm_bounding: sm_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      model_access_std_model: global_entity_component_of::<SceneModelStdModelRenderPayload>()
        .read_foreign_key(),
      std_model_access_mesh: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      mesh_vertex_refs: mesh_vertex_refs.expect_resolve_stage().into_boxed_multi(),
      semantic: global_entity_component_of::<AttributesMeshEntityVertexBufferSemantic>().read(),
      mesh_index_attribute:
        global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
          .read_foreign_key(),
      mesh_topology: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
      buffer: global_entity_component_of::<BufferEntityData>().read(),
      vertex_buffer_ref: global_entity_component_of::<SceneBufferViewBufferId<AttributeVertexRef>>(
      )
      .read_foreign_key(),
    };

    let wide_line_picker = WideLinePicker {
      lines: global_entity_component_of::<WideLineMeshBuffer>().read(),
      relation: global_entity_component_of::<SceneModelWideLineRenderPayload>().read_foreign_key(),
      sm_bounding: wide_line_sm_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
    };

    let local_model_pickers: Vec<Box<dyn LocalModelPicker>> =
      vec![Box::new(att_mesh_picker), Box::new(wide_line_picker)];

    let scene_model_picker = SceneModelPickerBaseImpl {
      internal: local_model_pickers,
      scene_model_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_world: node_world
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      node_net_visible: node_net_visible
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
    };

    ViewerPicker::new(
      Box::new(scene_model_picker),
      cx.input,
      camera_view_projection_inv,
    )
    .into()
  } else {
    None
  }
}

impl ViewerPicker {
  pub fn new(
    scene_model_picker: Box<dyn SceneModelPicker>,
    input: &PlatformEventInput,
    camera_view_projection_inv: Mat4<f64>,
  ) -> Self {
    let mouse_position = &input.window_state.mouse_position;
    let window_size = &input.window_state.physical_size;

    let normalized_position_ndc =
      compute_normalized_position_in_canvas_coordinate(*mouse_position, *window_size);

    let normalized_position_ndc: Vec2<f32> = normalized_position_ndc.into();
    let normalized_position_ndc_f64 = normalized_position_ndc.into_f64();
    let current_mouse_ray_in_world =
      cast_world_ray(camera_view_projection_inv, normalized_position_ndc_f64);

    ViewerPicker {
      scene_model_picker,
      current_mouse_ray_in_world,
      conf: Default::default(),
      normalized_position: Vec2::from((
        mouse_position.0 / window_size.0,
        mouse_position.1 / window_size.1,
      )),
      normalized_position_ndc,
      camera_view_size: Size::from_f32_pair_min_one(input.window_state.physical_size),
    }
  }

  pub fn current_mouse_ray_in_world(&self) -> Ray3<f64> {
    self.current_mouse_ray_in_world
  }

  pub fn normalized_position_ndc(&self) -> Vec2<f32> {
    self.normalized_position_ndc
  }

  pub fn normalized_position(&self) -> Vec2<f32> {
    self.normalized_position
  }
}

impl Picker3d for ViewerPicker {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
  ) -> Option<HitPoint3D<f64>> {
    self
      .scene_model_picker
      .query(
        model,
        &SceneRayQuery {
          world_ray,
          conf: self.conf.clone(),
          camera_view_size: self.camera_view_size,
        },
      )
      .map(|v| v.hit)
  }
}

pub fn prepare_picking_state<'a>(
  picker: &'a ViewerPicker,
  g: &WidgetSceneModelIntersectionGroupConfig,
) -> Interaction3dCtx<'a> {
  let world_ray_intersected_nearest = picker.pick_models_nearest(
    &mut g.group.iter().copied(),
    picker.current_mouse_ray_in_world,
  );

  Interaction3dCtx {
    normalized_mouse_position: picker.normalized_position,
    mouse_world_ray: picker.current_mouse_ray_in_world,
    picker: picker as &dyn Picker3d,
    world_ray_intersected_nearest,
  }
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
