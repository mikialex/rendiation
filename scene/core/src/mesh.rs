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

pub struct AttributeMeshEntityFromAttributeMeshDataWriter {
  buffer: EntityWriter<BufferEntity>,
  relation: EntityWriter<AttributeMeshVertexBufferRelation>,
  mesh: EntityWriter<AttributeMeshEntity>,
}

impl EntityCustomWrite<AttributeMeshEntity> for AttributesMesh {
  type Writer = AttributeMeshEntityFromAttributeMeshDataWriter;

  fn create_writer() -> Self::Writer {
    AttributeMeshEntityFromAttributeMeshDataWriter {
      buffer: global_entity_of::<BufferEntity>().entity_writer(),
      relation: global_entity_of::<AttributeMeshVertexBufferRelation>().entity_writer(),
      mesh: global_entity_of::<AttributeMeshEntity>().entity_writer(),
    }
  }

  fn write(self, writer: &mut Self::Writer) -> EntityHandle<AttributeMeshEntity> {
    let count = self.indices.as_ref().map(|(_, data)| data.count as u32);
    let index = self.indices.map(|(_, data)| data.write(&mut writer.buffer));

    let index = SceneBufferViewDataView {
      data: index,
      range: None,
      count,
    };

    let mesh_writer = &mut writer.mesh;
    index.write::<AttributeIndexRef, _>(mesh_writer);
    mesh_writer.component_value_writer::<AttributeMeshTopology>(self.mode);
    let mesh = mesh_writer.new_entity();

    for (semantic, vertex) in self.attributes {
      let count = vertex.count;
      let vertex = vertex.write(&mut writer.buffer);

      let vertex = SceneBufferViewDataView {
        data: Some(vertex),
        range: None,
        count: Some(count as u32),
      };

      let relation_writer = &mut writer.relation;
      vertex.write::<AttributeVertexRef, _>(relation_writer);
      relation_writer
        .component_value_writer::<AttributeMeshVertexBufferRelationRefAttributeMesh>(
          mesh.some_handle(),
        )
        .component_value_writer::<AttributeMeshVertexBufferSemantic>(semantic)
        .new_entity();
    }

    mesh
  }
}

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
