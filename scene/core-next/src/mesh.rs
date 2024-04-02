use crate::*;

declare_entity!(AttributeMeshEntity);
declare_entity!(VertexBufferEntity);

declare_entity!(AttributeMeshVertexBufferRelation);
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

pub fn register_mesh_data_model() {
  global_database().declare_entity::<AttributeMeshEntity>();

  global_database().declare_entity::<VertexBufferEntity>();

  global_database()
    .declare_entity::<AttributeMeshVertexBufferRelation>()
    .declare_foreign_key::<AttributeMeshVertexBufferRelationRefAttributeMesh>()
    .declare_foreign_key::<AttributeMeshVertexBufferRelationRefVertexBuffer>();
}
