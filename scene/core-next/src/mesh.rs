use rendiation_mesh_core::{AttributeSemantic, PrimitiveTopology};

use crate::*;

declare_entity!(AttributeMeshEntity);
declare_component!(
  AttributeMeshTopology,
  AttributeMeshEntity,
  PrimitiveTopology
);

declare_entity!(VertexBufferEntity);
declare_component!(
  VertexBufferData,
  VertexBufferEntity,
  Option<ExternalRefPtr<Vec<u8>>>
);

declare_entity!(AttributeMeshVertexBufferRelation);
declare_component!(
  AttributeMeshVertexBufferOffset,
  AttributeMeshVertexBufferRelation,
  u32
);
declare_component!(
  AttributeMeshVertexBufferSize,
  AttributeMeshVertexBufferRelation,
  u32
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
  VertexBufferEntity
);

pub fn register_attribute_mesh_data_model() {
  global_database()
    .declare_entity::<AttributeMeshEntity>()
    .declare_component::<AttributeMeshTopology>();

  global_database()
    .declare_entity::<VertexBufferEntity>()
    .declare_component::<VertexBufferData>();

  global_database()
    .declare_entity::<AttributeMeshVertexBufferRelation>()
    .declare_component::<AttributeMeshVertexBufferOffset>()
    .declare_component::<AttributeMeshVertexBufferSize>()
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
