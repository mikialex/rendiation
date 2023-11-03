use rendiation_geometry::Box3;

use crate::*;

mod optimization;
pub use optimization::*;

#[macro_export]
macro_rules! field_of {
  ($ty:ty =>$field:tt) => {
    |view: incremental::MaybeDeltaRef<'_, $ty>, send: &dyn Fn(&_)| match view {
      incremental::MaybeDeltaRef::All(value) => send(&value.$field),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field)
        }
      }
    }
  };
}

pub fn std_model_att_mesh_ref_change(
) -> impl ReactiveCollection<AllocIdx<StandardModel>, AllocIdx<AttributesMesh>> {
  storage_of::<StandardModel>()
    .single_listen_by_into_reactive_collection(|change, collector| {
      field_of!(StandardModel => mesh)(change, &|mesh| {
        if let MeshEnum::AttributesMesh(mesh) = mesh {
          collector(Some(AllocIdx::from(mesh.alloc_index())))
        } else {
          collector(None)
        }
      })
    })
    .collective_filter_map(|v| v)
}

pub fn attribute_boxes() -> impl ReactiveCollection<AllocIdx<AttributesMesh>, Box3<f32>> {
  storage_of::<AttributesMesh>()
    .single_listen_by_into_reactive_collection(any_change)
    .collective_map(|_| todo!())
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
    .single_listen_by_into_reactive_collection(|change, collector| {
      field_of!(SceneModelImpl => model)(change, &|mesh| {
        if let ModelEnum::Standard(mesh) = mesh {
          collector(Some(AllocIdx::from(mesh.alloc_index())))
        } else {
          collector(None)
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
  storage_of::<SceneModelImpl>().single_listen_by_into_reactive_collection(|change, collector| {
    field_of!(SceneModelImpl => node)(change, &|node| collector(node.guid()))
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
