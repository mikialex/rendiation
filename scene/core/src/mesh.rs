use crate::*;

declare_entity!(AttributesMeshEntity);
declare_component!(
  AttributesMeshEntityTopology,
  AttributesMeshEntity,
  PrimitiveTopology
);
declare_entity_associated!(AttributeIndexRef, AttributesMeshEntity);
impl SceneBufferView for AttributeIndexRef {}

declare_entity!(AttributesMeshEntityVertexBufferRelation);
declare_entity_associated!(AttributeVertexRef, AttributesMeshEntityVertexBufferRelation);
impl SceneBufferView for AttributeVertexRef {}

declare_component!(
  AttributesMeshEntityVertexBufferSemantic,
  AttributesMeshEntityVertexBufferRelation,
  AttributeSemantic
);

declare_foreign_key!(
  AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity,
  AttributesMeshEntityVertexBufferRelation,
  AttributesMeshEntity
);

pub struct AttributesMeshEntityFromAttributesMeshWriter {
  buffer: EntityWriter<BufferEntity>,
  relation: EntityWriter<AttributesMeshEntityVertexBufferRelation>,
  mesh: EntityWriter<AttributesMeshEntity>,
}

impl EntityCustomWrite<AttributesMeshEntity> for AttributesMesh {
  type Writer = AttributesMeshEntityFromAttributesMeshWriter;

  fn create_writer() -> Self::Writer {
    AttributesMeshEntityFromAttributesMeshWriter {
      buffer: global_entity_of::<BufferEntity>().entity_writer(),
      relation: global_entity_of::<AttributesMeshEntityVertexBufferRelation>().entity_writer(),
      mesh: global_entity_of::<AttributesMeshEntity>().entity_writer(),
    }
  }

  fn write(self, writer: &mut Self::Writer) -> EntityHandle<AttributesMeshEntity> {
    let count = self.indices.as_ref().map(|(_, data)| data.count as u32);
    let index = self.indices.map(|(_, data)| data.write(&mut writer.buffer));

    let index = SceneBufferViewDataView {
      data: index,
      range: None,
      count,
    };

    let mesh_writer = &mut writer.mesh;
    index.write::<AttributeIndexRef, _>(mesh_writer);
    mesh_writer.component_value_writer::<AttributesMeshEntityTopology>(self.mode);
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
        .component_value_writer::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(
          mesh.some_handle(),
        )
        .component_value_writer::<AttributesMeshEntityVertexBufferSemantic>(semantic)
        .new_entity();
    }

    mesh
  }
}

pub fn register_attribute_mesh_data_model() {
  let ecg = global_database()
    .declare_entity::<AttributesMeshEntity>()
    .declare_component::<AttributesMeshEntityTopology>();

  register_scene_buffer_view::<AttributeIndexRef>(ecg);

  global_database()
    .declare_entity::<BufferEntity>()
    .declare_component::<BufferEntityData>();

  let ecg = global_database()
    .declare_entity::<AttributesMeshEntityVertexBufferRelation>()
    .declare_component::<AttributesMeshEntityVertexBufferSemantic>()
    .declare_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>();

  register_scene_buffer_view::<AttributeVertexRef>(ecg);
}

declare_entity!(InstanceMeshInstanceEntity);
declare_component!(
  InstanceMeshWorldMatrix,
  InstanceMeshInstanceEntity,
  Mat4<f32>
);
declare_foreign_key!(
  InstanceMeshInstanceEntityRefAttributesMeshEntity,
  InstanceMeshInstanceEntity,
  AttributesMeshEntity
);

pub fn register_instance_mesh_data_model() {
  global_database()
    .declare_entity::<InstanceMeshInstanceEntity>()
    .declare_component::<InstanceMeshWorldMatrix>()
    .declare_foreign_key::<InstanceMeshInstanceEntityRefAttributesMeshEntity>();
}
