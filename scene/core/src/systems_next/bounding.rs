use rendiation_geometry::{Box3, SpaceBounding};

use crate::*;

pub fn std_model_att_mesh_ref_change(
) -> impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<AttributesMesh>> {
  storage_of::<StandardModel>()
    .listen_to_reactive_collection(|change| {
      field_of!(change, StandardModel => mesh).map(|mesh| {
        if let MeshEnum::AttributesMesh(mesh) = mesh {
          Some(AllocIdx::from(mesh.alloc_index()))
        } else {
          None
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn attribute_boxes() -> impl ReactiveCollection<AllocIdx<AttributesMesh>, Box3<f32>> {
  storage_of::<AttributesMesh>()
    .listen_to_reactive_collection(|_| Some(()))
    .collective_execute_map_by(|| {
      let box_compute = storage_of::<AttributesMesh>().create_key_mapper(|mesh| {
        mesh
          .read_shape()
          .primitive_iter()
          .map(|p| p.to_bounding())
          .collect()
      });
      move |k, _| box_compute(*k)
    })
}

pub fn model_attribute_boxes() -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
  attribute_boxes().one_to_many_fanout(std_model_att_mesh_ref_change().into_one_to_many_by_idx())
}

pub fn model_boxes(
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
  model_attribute_boxes().collective_select(foreign_mesh_local_box_support)
}

pub fn scene_model_std_model_ref_change(
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, AllocIdx<StandardModel>> {
  storage_of::<SceneModelImpl>()
    .listen_to_reactive_collection(|change| {
      field_of!(change, SceneModelImpl => model).map(|model| {
        if let ModelEnum::Standard(model) = model {
          Some(AllocIdx::from(model.alloc_index()))
        } else {
          None
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn scene_model_local_boxes(
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>> {
  model_boxes(foreign_mesh_local_box_support)
    .one_to_many_fanout(scene_model_std_model_ref_change().into_one_to_many_by_idx())
}

pub type NodeGUID = u64;
pub fn scene_model_node_ref_change() -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, NodeGUID>
{
  storage_of::<SceneModelImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneModelImpl => node).map(|node| node.guid())
  })
}

pub fn scene_model_world(
  node_world: impl ReactiveCollection<NodeGUID, Mat4<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>> {
  node_world.one_to_many_fanout(scene_model_node_ref_change().into_one_to_many_by_hash())
}

pub fn scene_model_world_box(
  node_world: impl ReactiveCollection<NodeGUID, Mat4<f32>>,
  foreign_mesh_local_box_support: impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>>,
  foreign_model_local_box_support: impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, Box3<f32>> {
  scene_model_local_boxes(foreign_mesh_local_box_support)
    .collective_select(foreign_model_local_box_support)
    .collective_intersect(scene_model_world(node_world))
    .collective_map(|(local_bbox, world_mat)| local_bbox.apply_matrix_into(world_mat))
}
