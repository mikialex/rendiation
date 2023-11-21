use crate::*;

#[global_registered_collection_and_many_one_relation]
pub fn scene_model_ref_std_model(
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

#[global_registered_collection_and_many_one_relation]
pub fn std_model_ref_att_mesh(
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

pub type NodeGUID = u64;
#[global_registered_collection]
pub fn scene_model_ref_node() -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, NodeGUID> {
  storage_of::<SceneModelImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneModelImpl => node).map(|node| node.guid())
  })
}
