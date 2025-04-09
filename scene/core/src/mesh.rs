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
  relation: EntityWriter<AttributesMeshEntityVertexBufferRelation>,
  mesh: EntityWriter<AttributesMeshEntity>,
}

pub trait AttributesMeshWriter {
  fn create_writer() -> AttributesMeshEntityFromAttributesMeshWriter;
  fn write(
    self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    buffer: &mut EntityWriter<BufferEntity>,
  ) -> AttributesMeshEntities;
}

pub struct AttributesMeshEntities {
  pub mesh: EntityHandle<AttributesMeshEntity>,
  pub index: Option<EntityHandle<BufferEntity>>,
  pub vertices: Vec<(
    EntityHandle<AttributesMeshEntityVertexBufferRelation>,
    EntityHandle<BufferEntity>,
  )>,
}

impl AttributesMeshEntities {
  pub fn clean_up(
    &self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    buffer: &mut EntityWriter<BufferEntity>,
  ) {
    for (r, b) in &self.vertices {
      writer.relation.delete_entity(*r);
      buffer.delete_entity(*b);
    }

    writer.mesh.delete_entity(self.mesh);

    if let Some(index) = self.index {
      buffer.delete_entity(index);
    }
  }
}

impl AttributesMeshWriter for AttributesMesh {
  fn create_writer() -> AttributesMeshEntityFromAttributesMeshWriter {
    AttributesMeshEntityFromAttributesMeshWriter {
      relation: global_entity_of::<AttributesMeshEntityVertexBufferRelation>().entity_writer(),
      mesh: global_entity_of::<AttributesMeshEntity>().entity_writer(),
    }
  }

  fn write(
    self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    buffer: &mut EntityWriter<BufferEntity>,
  ) -> AttributesMeshEntities {
    let count = self.indices.as_ref().map(|(_, data)| data.count as u32);
    let index_data = self.indices.map(|(_, data)| data.write(buffer));

    let index = SceneBufferViewDataView {
      data: index_data,
      range: None,
      count,
    };

    let mesh_writer = &mut writer.mesh;
    index.write::<AttributeIndexRef>(mesh_writer);
    mesh_writer.component_value_writer::<AttributesMeshEntityTopology>(self.mode);
    let mesh = mesh_writer.new_entity();

    let mut vertices = Vec::with_capacity(self.attributes.len());

    for (semantic, vertex) in self.attributes {
      let count = vertex.count;
      let vertex_data = vertex.write(buffer);

      let vertex = SceneBufferViewDataView {
        data: Some(vertex_data),
        range: None,
        count: Some(count as u32),
      };

      let relation_writer = &mut writer.relation;
      vertex.write::<AttributeVertexRef>(relation_writer);
      let vertex = relation_writer
        .component_value_writer::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(
          mesh.some_handle(),
        )
        .component_value_writer::<AttributesMeshEntityVertexBufferSemantic>(semantic)
        .new_entity();

      vertices.push((vertex, vertex_data));
    }

    AttributesMeshEntities {
      mesh,
      index: index_data,
      vertices,
    }
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

#[global_registered_query]
pub fn attribute_mesh_local_bounding(
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = Box3> {
  let PositionRelatedAttributeMeshQuery {
    indexed,
    none_indexed,
  } = attribute_mesh_position_query();

  fn get_ranged_buffer(buffer: &[u8], range: Option<BufferViewRange>) -> &[u8] {
    if let Some(range) = range {
      let start = range.offset as usize;
      let count = range
        .size
        .map(|v| u64::from(v) as usize)
        .unwrap_or(buffer.len());
      buffer.get(start..(start + count)).unwrap()
    } else {
      buffer
    }
  }

  let indexed = indexed
    .collective_execute_map_by(|| {
      let buffer_access = global_entity_component_of::<BufferEntityData>().read();
      move |_, ((position, position_range), (idx, idx_range, count))| {
        let index = buffer_access.get(idx).unwrap();
        let index = get_ranged_buffer(index, idx_range);

        let count = count as usize;
        let index = if index.len() / count == 2 {
          let index: &[u16] = cast_slice(index);
          DynIndexRef::Uint16(index)
        } else if index.len() / count == 4 {
          let index: &[u32] = cast_slice(index);
          DynIndexRef::Uint32(index)
        } else {
          unreachable!("index count must be 2 or 4 bytes")
        };

        let position = buffer_access.get(position.unwrap()).unwrap();
        let position = get_ranged_buffer(position, position_range);
        let position: &[Vec3<f32>] = cast_slice(position);

        // as we are compute bounding, the topology not matters
        let mesh = IndexedMesh::<TriangleList, _, _>::new(position, index);
        mesh
          .primitive_iter()
          .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
      }
    })
    .into_boxed();

  let none_indexed = none_indexed
    .collective_execute_map_by(|| {
      let buffer_access = global_entity_component_of::<BufferEntityData>().read();
      move |_, (position, position_range)| {
        let position = buffer_access.get(position.unwrap()).unwrap();
        let position = get_ranged_buffer(position, position_range);
        let position: &[Vec3<f32>] = cast_slice(position);

        // as we are compute bounding, the topology not matters
        let mesh = NoneIndexedMesh::<TriangleList, _>::new(position);
        mesh
          .primitive_iter()
          .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
      }
    })
    .into_boxed();

  indexed.collective_select(none_indexed)
}

// todo, this should be registered into global query registry
pub fn attribute_mesh_position_query() -> PositionRelatedAttributeMeshQuery {
  let index_buffer_ref =
    global_watch().watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  let ranged_index_buffer = index_buffer_ref
    .collective_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .into_forker();

  let index_count = global_watch().watch::<SceneBufferViewBufferItemCount<AttributeIndexRef>>();

  let indexed_meshes_and_its_range = ranged_index_buffer
    .clone()
    .collective_zip(index_count)
    .collective_filter_map(|((index, range), count)| index.map(|i| (i, range, count.unwrap())));

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

  let ranged_position_buffer = vertex_buffer_ref
    .collective_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .into_boxed();

  let ab_ref_mesh = global_watch()
    .watch_typed_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .collective_filter_map(|v| v)
    .filter_by_keyset(positions_scope)
    .hash_reverse_assume_one_one()
    .into_boxed();

  // todo, impl chain instead of using one to many fanout
  let attribute_mesh_access_position_buffer = ranged_position_buffer
    .one_to_many_fanout(ab_ref_mesh.into_one_to_many_by_hash())
    .into_boxed();

  let attribute_mesh_access_position_buffer = attribute_mesh_access_position_buffer.into_forker();

  let none_indexed = attribute_mesh_access_position_buffer
    .clone()
    .filter_by_keyset(none_indexed_mesh_set)
    .into_boxed();

  let indexed = attribute_mesh_access_position_buffer
    .collective_intersect(indexed_meshes_and_its_range)
    .into_boxed();

  PositionRelatedAttributeMeshQuery {
    none_indexed,
    indexed,
  }
}

pub struct PositionRelatedAttributeMeshQuery {
  pub indexed: BoxedDynReactiveQuery<
    EntityHandle<AttributesMeshEntity>,
    (
      (Option<EntityHandle<BufferEntity>>, Option<BufferViewRange>), // position
      (EntityHandle<BufferEntity>, Option<BufferViewRange>, u32), /* index, count(used to distinguish the index format) */
    ),
  >,
  pub none_indexed: BoxedDynReactiveQuery<
    EntityHandle<AttributesMeshEntity>,
    (Option<EntityHandle<BufferEntity>>, Option<BufferViewRange>),
  >,
}
