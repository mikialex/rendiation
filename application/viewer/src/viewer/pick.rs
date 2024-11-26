use database::global_entity_component_of;
use rendiation_gui_3d::*;
use rendiation_mesh_core::MeshBufferIntersectConfig;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct ViewerPicker {
  current_mouse_ray_in_world: Ray3,
  conf: MeshBufferIntersectConfig,
  camera_view_size: Size,
  scene_model_picker: SceneModelPickerImpl,
}

impl ViewerPicker {
  pub fn new(
    dep: &Viewer3dSceneDerive,
    input: &PlatformEventInput,
    camera_id: EntityHandle<SceneCameraEntity>,
  ) -> Self {
    let scene_model_picker = SceneModelPickerImpl {
      mesh_bounding: dep.mesh_local_bounding.clone(),
      scene_model_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      model_access_std_model: global_entity_component_of::<SceneModelStdModelRenderPayload>()
        .read_foreign_key(),
      std_model_access_mesh: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      mesh_vertex_refs: dep.mesh_vertex_ref.clone(),
      semantic: global_entity_component_of::<AttributesMeshEntityVertexBufferSemantic>().read(),
      mesh_index_attribute:
        global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
          .read_foreign_key(),
      mesh_topology: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
      buffer: global_entity_component_of::<BufferEntityData>().read(),
      vertex_buffer_ref: global_entity_component_of::<SceneBufferViewBufferId<AttributeVertexRef>>(
      )
      .read_foreign_key(),
      node_world: dep.world_mat.clone(),
      node_net_visible: dep.node_net_visible.clone(),
    };

    let mouse_position = &input.window_state.mouse_position;
    let window_size = &input.window_state.size;

    let normalized_position =
      compute_normalized_position_in_canvas_coordinate(*mouse_position, *window_size);

    let projection_inv = dep
      .camera_transforms
      .access(&camera_id)
      .unwrap()
      .view_projection_inv;

    let current_mouse_ray_in_world = cast_world_ray(projection_inv, normalized_position.into());

    ViewerPicker {
      scene_model_picker,
      current_mouse_ray_in_world,
      conf: Default::default(),
      camera_view_size: Size::from_f32_pair_min_one(input.window_state.size),
    }
  }

  pub fn current_mouse_ray_in_world(&self) -> &Ray3 {
    &self.current_mouse_ray_in_world
  }
}

impl Picker3d for ViewerPicker {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<HitPoint3D> {
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

pub fn prepare_picking_state(
  picker: ViewerPicker,
  g: &WidgetSceneModelIntersectionGroupConfig,
) -> Interaction3dCtx {
  let world_ray_intersected_nearest = picker.pick_models_nearest(
    &mut g.group.iter().copied(),
    picker.current_mouse_ray_in_world,
  );

  Interaction3dCtx {
    mouse_world_ray: picker.current_mouse_ray_in_world,
    picker: Box::new(picker),
    world_ray_intersected_nearest,
  }
}

pub fn compute_normalized_position_in_canvas_coordinate(
  offset: (f32, f32),
  size: (f32, f32),
) -> (f32, f32) {
  (offset.0 / size.0 * 2. - 1., -(offset.1 / size.1 * 2. - 1.))
}
