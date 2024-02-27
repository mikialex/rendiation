use crate::*;

#[global_registered_collection_and_many_one_idx_relation]
pub fn scene_model_ref_std_model(
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, AllocIdx<StandardModel>> {
  storage_of::<SceneModelImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneModelImpl => model).cared_then(|model| {
      if let ModelEnum::Standard(model) = model {
        Some(AllocIdx::from(model.alloc_index()))
      } else {
        None
      }
    })
  })
}

#[global_registered_collection_and_many_one_idx_relation]
pub fn std_model_ref_att_mesh(
) -> impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<AttributesMesh>> {
  storage_of::<StandardModel>().listen_to_reactive_collection(|change| {
    field_of!(change, StandardModel => mesh).cared_then(|mesh| {
      if let MeshEnum::AttributesMesh(mesh) = mesh {
        Some(AllocIdx::from(mesh.alloc_index()))
      } else {
        None
      }
    })
  })
}

#[global_registered_collection_and_many_one_hash_relation]
pub fn scene_model_ref_node() -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, NodeIdentity> {
  storage_of::<SceneModelImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneModelImpl => node).cared_map(|node| node.scene_and_node_id())
  })
}

#[global_registered_collection_and_many_one_hash_relation]
pub fn scene_light_ref_node() -> impl ReactiveCollection<AllocIdx<SceneLightImpl>, NodeIdentity> {
  storage_of::<SceneLightImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneLightImpl => node).cared_map(|node| node.scene_and_node_id())
  })
}

#[global_registered_collection_and_many_one_hash_relation]
pub fn scene_camera_ref_node() -> impl ReactiveCollection<AllocIdx<SceneCameraImpl>, NodeIdentity> {
  storage_of::<SceneCameraImpl>().listen_to_reactive_collection(|change| {
    field_of!(change, SceneCameraImpl => node).cared_map(|node| node.scene_and_node_id())
  })
}

pub trait DowncastFromMaterialEnum: IncrementalBase {
  fn downcast_from_material_enum(mat: &MaterialEnum) -> Option<&IncrementalSignalPtr<Self>>;
}
macro_rules! material_enum_cast {
  ($MaterialTy: ty, $EnumName: tt) => {
    impl DowncastFromMaterialEnum for $MaterialTy {
      fn downcast_from_material_enum(mat: &MaterialEnum) -> Option<&IncrementalSignalPtr<Self>> {
        match mat {
          MaterialEnum::$EnumName(m) => Some(m),
          _ => None,
        }
      }
    }
  };
}
material_enum_cast!(FlatMaterial, Flat);
material_enum_cast!(PhysicalMetallicRoughnessMaterial, PhysicalMetallicRoughness);
material_enum_cast!(
  PhysicalSpecularGlossinessMaterial,
  PhysicalSpecularGlossiness
);

fn global_std_model_ref_material_impl<M: DowncastFromMaterialEnum>(
) -> impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<M>> {
  storage_of::<StandardModel>().listen_to_reactive_collection(|change| {
    field_of!(change, StandardModel => material).cared_then(|mat| {
      M::downcast_from_material_enum(mat).map(|m| AllocIdx::from(m.alloc_index()))
    })
  })
}

pub fn global_std_model_ref_material<M: DowncastFromMaterialEnum>(
) -> impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<M>> + Clone {
  global_collection_registry().get_or_create_relation_by_idx(global_std_model_ref_material_impl)
}

pub fn global_material_relations<M: DowncastFromMaterialEnum>(
) -> impl ReactiveOneToManyRelationship<AllocIdx<M>, AllocIdx<StandardModel>> + Clone {
  global_collection_registry().get_or_create_relation_by_idx(global_std_model_ref_material_impl)
}
