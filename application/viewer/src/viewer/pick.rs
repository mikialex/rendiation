use database::global_entity_component_of;
use rendiation_gui_3d::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct ViewerPicker {
  scene_model_picker: SceneModelPickerImpl,
}

impl ViewerPicker {
  pub fn new(dep: &Viewer3dSceneDerive) -> Self {
    let scene_model_picker = SceneModelPickerImpl {
      scene_model_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      model_access_std_model: global_entity_component_of::<SceneModelStdModelRenderPayload>()
        .read_foreign_key(),
      std_model_access_mesh: global_entity_component_of::<StandardModelRefAttributeMesh>()
        .read_foreign_key(),
      mesh_vertex_refs: dep.mesh_vertex_ref.clone(),
      semantic: global_entity_component_of::<AttributeMeshVertexBufferSemantic>().read(),
      mesh_index_attribute:
        global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
          .read_foreign_key(),
      mesh_topology: global_entity_component_of::<AttributeMeshTopology>().read(),
      buffer: global_entity_component_of::<BufferEntityData>().read(),
      vertex_buffer_ref: global_entity_component_of::<SceneBufferViewBufferId<AttributeVertexRef>>(
      )
      .read_foreign_key(),
      node_world: dep.world_mat.clone(),
      node_net_visible: dep.node_net_visible.clone(),
    };
    ViewerPicker { scene_model_picker }
  }
}

impl Picker3d for ViewerPicker {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>> {
    self.scene_model_picker.query(
      model,
      &SceneRayQuery {
        world_ray,
        conf: todo!(),
        camera_view_size: todo!(),
      },
    );
    todo!()
  }
}

struct Interaction3dCtxProvider {}

impl Interaction3dCtxProvider {
  pub fn compute_picking_state(
    picker: &ViewerPicker,
    input: PlatformEventInput,
  ) -> Interaction3dCtx {
    let mouse_position = &input.window_state.mouse_position;
    let window_size = &input.window_state.size;

    let normalized_position =
      compute_normalized_position_in_canvas_coordinate(*mouse_position, *window_size);

    Interaction3dCtx {
      picker: todo!(),
      mouse_world_ray: todo!(),
      intersection_group: todo!(),
      world_ray_intersected_nearest: todo!(),
    }
  }
}

pub fn compute_normalized_position_in_canvas_coordinate(
  offset: (f32, f32),
  size: (f32, f32),
) -> (f32, f32) {
  (offset.0 / size.0 * 2. - 1., -(offset.1 / size.1 * 2. - 1.))
}
