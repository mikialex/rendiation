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

#[global_registered_collection]
pub fn attribute_mesh_local_bounding(
) -> impl ReactiveCollection<Key = EntityHandle<AttributesMeshEntity>, Value = Box3> {
  let index_buffer_ref =
    global_watch().watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  let ranged_index_buffer = index_buffer_ref
    .collective_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .into_forker();

  let indexed_meshes_and_its_range = ranged_index_buffer
    .clone()
    .collective_filter_map(|(index, range)| index.map(|i| (i, range)));

  let none_indexed_mesh_set =
    ranged_index_buffer.collective_filter_map(|(b, _)| b.is_none().then_some(()));

  let positions_scope = global_watch()
    .watch::<AttributesMeshEntityVertexBufferSemantic>()
    .collective_filter(|semantic| semantic == AttributeSemantic::Positions)
    .collective_map(|_| {})
    .into_forker();

  let vertex_buffer_ref = global_watch()
    .watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .filter_by_keyset(positions_scope.clone());

  let vertex_buffer_range = global_watch()
    .watch::<SceneBufferViewBufferRange<AttributeVertexRef>>()
    .filter_by_keyset(positions_scope.clone());

  let ranged_position_buffer =
    vertex_buffer_ref.collective_union(vertex_buffer_range, |(a, b)| Some((a?, b?)));

  let ab_ref_mesh = global_watch()
    .watch_typed_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .collective_filter_map(|v| v)
    .filter_by_keyset(positions_scope)
    .hash_reverse_assume_one_one();

  // todo, impl chain instead of using one to many fanout
  let attribute_mesh_access_position_buffer =
    ranged_position_buffer.one_to_many_fanout(ab_ref_mesh.into_one_to_many_by_hash());

  let attribute_mesh_access_position_buffer = attribute_mesh_access_position_buffer.into_forker();

  let compute_none_indexed_bounding = attribute_mesh_access_position_buffer
    .clone()
    .filter_by_keyset(none_indexed_mesh_set)
    .collective_map(|_| Box3::<f32>::empty());

  let compute_indexed_bounding = attribute_mesh_access_position_buffer
    .collective_intersect(indexed_meshes_and_its_range)
    .collective_map(|_| Box3::<f32>::empty());

  compute_none_indexed_bounding.collective_select(compute_indexed_bounding)
}
