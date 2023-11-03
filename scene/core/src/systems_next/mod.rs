use rendiation_geometry::Box3;

use crate::*;

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
  storage_of::<StandardModel>().single_listen_by_into_reactive_collection(|change, collector| {
    field_of!(StandardModel => mesh)(change, &|mesh| {
      if let MeshEnum::AttributesMesh(mesh) = mesh {
        collector(AllocIdx::from(mesh.alloc_index()))
      }
    })
  })
}

pub fn attribute_boxes() -> impl ReactiveCollection<AllocIdx<AttributesMesh>, Box3<f32>> {
  storage_of::<AttributesMesh>()
    .single_listen_by_into_reactive_collection(any_change)
    .collective_map(|_| todo!())
}

pub fn model_attribute_boxes() -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
  attribute_boxes().one_to_many_fanout(std_model_att_mesh_ref_change().into_one_to_many_by_idx())
}

// pub fn model_boxes() -> impl ReactiveCollection<AllocIdx<StandardModel>, Box3<f32>> {
//   //
// }

// pub fn model_boxes() -> impl ReactiveCollection<AllocIdx<SceneModel>, Box3<f32>> {
//   //
// }

// pub fn model_node() -> impl ReactiveCollection<AllocIdx<SceneModel>, AllocIdx<SceneNode>> {
//   //
// }

// pub fn node_world() -> impl ReactiveCollection<AllocIdx<SceneNode>, Mat4<f32>> {
//   //
// }

// pub fn model_world() -> impl ReactiveCollection<AllocIdx<SceneModel>, Mat4<f32>> {
//   //
// }

// pub fn model_world_box() -> impl ReactiveCollection<AllocIdx<SceneModel>, Box3<f32>> {
//   //
// }
