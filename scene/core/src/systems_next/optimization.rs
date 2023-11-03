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
    .single_listen_by_into_reactive_collection(|change, collector| {
      field_of!(StandardModel => material)(change, &|material| {
        if let MaterialEnum::PhysicalMetallicRoughness(material) = material {
          collector(Some(material.guid()))
        } else {
          collector(None)
        }
      })
    })
    .collective_filter_map(|v| v)
}
pub fn std_model_physical_sg_ref_change() -> impl ReactiveCollection<AllocIdx<StandardModel>, u64> {
  storage_of::<StandardModel>()
    .single_listen_by_into_reactive_collection(|change, collector| {
      field_of!(StandardModel => material)(change, &|material| {
        if let MaterialEnum::PhysicalSpecularGlossiness(material) = material {
          collector(Some(material.guid()))
        } else {
          collector(None)
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn scene_model_material_ids(
  foreign_materials: impl ReactiveCollection<AllocIdx<StandardModel>, u64>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, u64> {
  std_model_material_ids(foreign_materials)
    .one_to_many_fanout(scene_model_std_model_ref_change().into_one_to_many_by_idx())
}

pub fn optimizable_std_model() -> impl ReactiveCollection<AllocIdx<StandardModel>, ()> {
  storage_of::<StandardModel>()
    .single_listen_by_into_reactive_collection(|change, collector| {
      field_of!(StandardModel => mesh)(change, &|mesh| {
        if let MeshEnum::AttributesMesh(_) = mesh {
          collector(Some(())) // todo, check if attribute mesh is correct type
        } else {
          collector(None)
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn instance_mapping(
  node_merge_key: impl ReactiveCollection<NodeGUID, u64>,
  foreign_materials: impl ReactiveCollection<AllocIdx<StandardModel>, u64>,
) -> impl ReactiveOneToManyRelationship<InstanceKey, AllocIdx<SceneModelImpl>> {
  let optimizable_scene_model = optimizable_std_model()
    .one_to_many_fanout(scene_model_std_model_ref_change().into_one_to_many_by_idx()); // todo, fork relation

  let scene_model_node_merge_key =
    node_merge_key.one_to_many_fanout(scene_model_node_ref_change().into_one_to_many_by_hash());

  optimizable_scene_model
    .collective_intersect(scene_model_node_merge_key)
    .collective_intersect(scene_model_material_ids(foreign_materials))
    .collective_map(|(((), node_key), material_guid)| InstanceKey {
      material_guid,
      node_key,
    })
    .into_one_to_many_by_hash()
}
