use rendiation_mesh_core::{AttributeSemantic, PrimitiveTopology};

use crate::*;

declare_entity!(AttributeMeshEntity);
declare_component!(
  AttributeMeshTopology,
  AttributeMeshEntity,
  PrimitiveTopology
);
declare_entity_associated!(AttributeIndexRef, AttributeMeshEntity);
impl SceneBufferView for AttributeIndexRef {}

declare_entity!(AttributeMeshVertexBufferRelation);
declare_entity_associated!(AttributeVertexRef, AttributeMeshVertexBufferRelation);
impl SceneBufferView for AttributeVertexRef {}

declare_component!(
  AttributeMeshVertexBufferSemantic,
  AttributeMeshVertexBufferRelation,
  AttributeSemantic
);

declare_foreign_key!(
  AttributeMeshVertexBufferRelationRefAttributeMesh,
  AttributeMeshVertexBufferRelation,
  AttributeMeshEntity
);

pub fn register_attribute_mesh_data_model() {
  let ecg = global_database()
    .declare_entity::<AttributeMeshEntity>()
    .declare_component::<AttributeMeshTopology>();

  register_scene_buffer_view::<AttributeIndexRef>(ecg);

  global_database()
    .declare_entity::<BufferEntity>()
    .declare_component::<BufferEntityData>();

  let ecg = global_database()
    .declare_entity::<AttributeMeshVertexBufferRelation>()
    .declare_component::<AttributeMeshVertexBufferSemantic>()
    .declare_foreign_key::<AttributeMeshVertexBufferRelationRefAttributeMesh>();

  register_scene_buffer_view::<AttributeVertexRef>(ecg);
}

declare_entity!(InstanceMeshInstanceEntity);
declare_component!(
  InstanceMeshWorldMatrix,
  InstanceMeshInstanceEntity,
  Mat4<f32>
);
declare_foreign_key!(
  InstanceMeshInstanceEntityRefAttributeMesh,
  InstanceMeshInstanceEntity,
  AttributeMeshEntity
);

pub fn register_instance_mesh_data_model() {
  global_database()
    .declare_entity::<InstanceMeshInstanceEntity>()
    .declare_component::<InstanceMeshWorldMatrix>()
    .declare_foreign_key::<InstanceMeshInstanceEntityRefAttributeMesh>();
}
