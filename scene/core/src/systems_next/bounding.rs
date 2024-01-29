use rendiation_geometry::{Box3, SpaceBounding};

use crate::*;

pub type SceneModelWorldBoundingGetter<'a> =
  &'a dyn Fn(&AllocIdx<SceneModelImpl>) -> Option<Box3<f32>>;

pub fn attribute_boxes() -> impl ReactiveCollection<AllocIdx<AttributesMesh>, Box3<f32>> {
  storage_of::<AttributesMesh>()
    .listen_all_instance_changed_set()
    .collective_execute_simple_map(|mesh| {
      mesh
        .read_shape()
        .primitive_iter()
        .map(|p| p.to_bounding())
        .collect()
    })
}

pub fn model_attribute_boxes() -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
  attribute_boxes().one_to_many_fanout(std_model_ref_att_mesh_many_one_relation())
}

pub fn model_boxes(
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
  model_attribute_boxes().collective_select(foreign_mesh_local_box_support)
}

pub fn scene_model_local_boxes(
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>> {
  model_boxes(foreign_mesh_local_box_support)
    .one_to_many_fanout(scene_model_ref_std_model_many_one_relation())
}

pub fn scene_model_world(
  node_world: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>> {
  node_world.one_to_many_fanout(scene_model_ref_node_many_one_relation())
}

pub fn scene_model_world_box(
  node_world: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
  foreign_model_local_box_support: impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>> {
  scene_model_local_boxes(foreign_mesh_local_box_support)
    .collective_select(foreign_model_local_box_support)
    .collective_intersect(scene_model_world(node_world))
    .collective_map(|(local_bbox, world_mat)| local_bbox.apply_matrix_into(world_mat))
}
