use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceKey {
  material_guid: u64,
  node_key: u64,
}

pub fn std_model_material_ids(
  foreign_materials: impl ReactiveCollection<AllocIdx<StandardModel>, u64>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, u64> {
  std_model_physical_mr_ref_change()
    .collective_select(std_model_physical_sg_ref_change())
    .collective_select(foreign_materials)
}

pub fn std_model_physical_mr_ref_change() -> impl ReactiveCollection<AllocIdx<StandardModel>, u64> {
  storage_of::<StandardModel>()
    .listen_to_reactive_collection(|change| {
      field_of!(change, StandardModel => material).map(|material| {
        if let MaterialEnum::PhysicalMetallicRoughness(material) = material {
          Some(material.guid())
        } else {
          None
        }
      })
    })
    .collective_filter_map(|v| v)
}
pub fn std_model_physical_sg_ref_change() -> impl ReactiveCollection<AllocIdx<StandardModel>, u64> {
  storage_of::<StandardModel>()
    .listen_to_reactive_collection(|change| {
      field_of!(change, StandardModel => material).map(|material| {
        if let MaterialEnum::PhysicalSpecularGlossiness(material) = material {
          Some(material.guid())
        } else {
          None
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn scene_model_material_ids(
  foreign_materials: impl ReactiveCollection<AllocIdx<StandardModel>, u64>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, u64> {
  std_model_material_ids(foreign_materials)
    .one_to_many_fanout(scene_model_ref_std_model_many_one_relation())
}

pub fn optimizable_std_model() -> impl ReactiveCollection<AllocIdx<StandardModel>, ()> {
  storage_of::<StandardModel>()
    .listen_to_reactive_collection(|change| {
      field_of!(change, StandardModel => mesh).map(|mesh| {
        if let MeshEnum::AttributesMesh(_) = mesh {
          Some(()) // todo, check if attribute mesh is correct type
        } else {
          None
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn instance_mapping(
  node_merge_key: impl ReactiveCollection<NodeIdentity, u64>,
  foreign_materials: impl ReactiveCollection<AllocIdx<StandardModel>, u64>,
) -> impl ReactiveOneToManyRelationship<InstanceKey, AllocIdx<SceneModelImpl>> {
  let optimizable_scene_model =
    optimizable_std_model().one_to_many_fanout(scene_model_ref_std_model_many_one_relation());

  let scene_model_node_merge_key =
    node_merge_key.one_to_many_fanout(scene_model_ref_node_many_one_relation());

  optimizable_scene_model
    .collective_intersect(scene_model_node_merge_key)
    .collective_intersect(scene_model_material_ids(foreign_materials))
    .collective_map(|(((), node_key), material_guid)| InstanceKey {
      material_guid,
      node_key,
    })
    .into_one_to_many_by_hash()
}

pub fn selected_scene() -> impl ReactiveCollection<AllocIdx<Scene>, ()> {}

pub fn selected_scene_models(
  s_sm: impl ReactiveCollection<AllocIdx<SceneModelImpl>, AllocIdx<Scene>>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, ()> {
  selected_scene().one_to_many_fanout(s_sm.into_one_to_many_by_idx())
}

pub fn selected_scene_std_models(
  s_sm: impl ReactiveCollection<AllocIdx<SceneModelImpl>, AllocIdx<Scene>>,
  sm_std: impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<SceneModelImpl>>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, ()> {
  selected_scene_models(s_sm).one_to_many_fanout(sm_std.into_one_to_many_by_idx())
}

// pub fn selected_scene_std_models() -> impl ReactiveCollection<AllocIdx<StandardModel>, ()> {

// }

// pub fn selected_scene_std_models() -> impl ReactiveCollection<AllocIdx<StandardModel>, ()> {

// }
