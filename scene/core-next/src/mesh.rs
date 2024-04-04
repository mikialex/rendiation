use rendiation_mesh_core::{AttributeSemantic, BufferViewRange, PrimitiveTopology};

use crate::*;

declare_entity!(AttributeMeshEntity);
declare_component!(
  AttributeMeshTopology,
  AttributeMeshEntity,
  PrimitiveTopology
);
declare_foreign_key!(AttributeMeshIndex, AttributeMeshEntity, BufferEntity);
declare_component!(
  AttributeMeshIndexBufferRange,
  AttributeMeshEntity,
  Option<BufferViewRange>
);

declare_entity!(AttributeMeshVertexBufferRelation);
declare_component!(
  AttributeMeshVertexBufferRange,
  AttributeMeshVertexBufferRelation,
  BufferViewRange
);
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
declare_foreign_key!(
  AttributeMeshVertexBufferRelationRefVertexBuffer,
  AttributeMeshVertexBufferRelation,
  BufferEntity
);

pub fn register_attribute_mesh_data_model() {
  global_database()
    .declare_entity::<AttributeMeshEntity>()
    .declare_component::<AttributeMeshTopology>()
    .declare_foreign_key::<AttributeMeshIndex>()
    .declare_component::<AttributeMeshIndexBufferRange>();

  global_database()
    .declare_entity::<BufferEntity>()
    .declare_component::<BufferEntityData>();

  global_database()
    .declare_entity::<AttributeMeshVertexBufferRelation>()
    .declare_component::<AttributeMeshVertexBufferRange>()
    .declare_component::<AttributeMeshVertexBufferSemantic>()
    .declare_foreign_key::<AttributeMeshVertexBufferRelationRefAttributeMesh>()
    .declare_foreign_key::<AttributeMeshVertexBufferRelationRefVertexBuffer>();
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
