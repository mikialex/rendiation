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

impl AttributesMeshEntityFromAttributesMeshWriter {
  pub fn notify_reserve_changes(&mut self, size: usize, buffer: &mut EntityWriter<BufferEntity>) {
    self.relation.notify_reserve_changes(size * 3); // assume 3 attributes
    buffer.notify_reserve_changes(size * 3);
    self.mesh.notify_reserve_changes(size);
  }
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

const CHECK_NORMAL: bool = false;

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
      if CHECK_NORMAL && semantic == AttributeSemantic::Normals {
        let vertex = vertex.read();
        for normal in vertex.visit_slice::<Vec3<f32>>().unwrap() {
          if (normal.length() - 1.0).abs() > 0.01 {
            println!("normal length error: {}", normal.length());
          }
        }
      }

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

pub struct AttributeMeshLocalBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for AttributeMeshLocalBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3> + 'static; // attribute mesh entity

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    use_attribute_mesh_local_bounding(cx)
  }
}

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

fn use_attribute_mesh_local_bounding(
  cx: &mut impl DBHookCxLike,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3>> {
  let (position_source1, position_source2) = use_attribute_mesh_position_query(cx).fork();

  let indexed = position_source1
    .map(|v| v.indexed)
    .use_dual_query_execute_map(cx, || {
      let buffer_access = get_db_view::<BufferEntityData>();
      move |_, ((position, position_range), (idx, idx_range, count))| {
        let index = buffer_access.access(&idx).unwrap();
        let index = get_ranged_buffer(&index, idx_range);

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

        let position = buffer_access.access(&position.unwrap()).unwrap();
        let position = get_ranged_buffer(&position, position_range);
        let position: &[Vec3<f32>] = cast_slice(position);

        // as we are compute bounding, the topology not matters
        let mesh = IndexedMesh::<TriangleList, _, _>::new(position, index);
        mesh
          .primitive_iter()
          .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
      }
    });

  let none_indexed = position_source2
    .map(|v| v.none_indexed)
    .use_dual_query_execute_map(cx, || {
      let buffer_access = get_db_view::<BufferEntityData>();
      move |_, (position, position_range)| {
        let position = buffer_access.access(&position.unwrap()).unwrap();
        let position = get_ranged_buffer(&position, position_range);
        let position: &[Vec3<f32>] = cast_slice(position);

        // as we are compute bounding, the topology not matters
        let mesh = NoneIndexedMesh::<TriangleList, _>::new(position);
        mesh
          .primitive_iter()
          .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
      }
    });

  indexed.dual_query_select(none_indexed)
}

// todo, this should be registered into global query registry
pub fn use_attribute_mesh_position_query(
  cx: &mut impl DBHookCxLike,
) -> UseResult<PositionRelatedAttributeMeshQuery> {
  let index_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  let (ranged_index_buffer1, ranged_index_buffer2) = index_buffer_ref
    .dual_query_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_boxed()
    .fork();

  let index_count = cx.use_dual_query::<SceneBufferViewBufferItemCount<AttributeIndexRef>>();

  let indexed_meshes_and_its_range = ranged_index_buffer1
    .dual_query_zip(index_count)
    .dual_query_filter_map(|((index, range), count)| index.map(|i| (i, range, count.unwrap())))
    .dual_query_boxed();

  let none_indexed_mesh_set =
    ranged_index_buffer2.dual_query_filter_map(|(b, _)| b.is_none().then_some(()));

  let (positions_scope1, positions_scope2) = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferSemantic>()
    .dual_query_filter_map(|semantic| (semantic == AttributeSemantic::Positions).then_some(()))
    .dual_query_boxed()
    .fork();

  let (positions_scope2, positions_scope3) = positions_scope2.fork();

  let vertex_buffer_ref = cx
    .use_dual_query::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .dual_query_filter_by_set(positions_scope1)
    .dual_query_boxed();

  let vertex_buffer_range = cx
    .use_dual_query::<SceneBufferViewBufferRange<AttributeVertexRef>>()
    .dual_query_filter_by_set(positions_scope2)
    .dual_query_boxed();

  let ranged_position_buffer = vertex_buffer_ref
    .dual_query_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_boxed();

  let ab_ref_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_filter_map(|v| v)
    .dual_query_boxed()
    .dual_query_filter_by_set(positions_scope3)
    .use_dual_query_hash_reverse_checked_one_one(cx)
    .dual_query_boxed();

  // todo, impl chain instead of using one to many fanout
  let attribute_mesh_access_position_buffer = ranged_position_buffer
    .fanout(ab_ref_mesh.use_dual_query_hash_many_to_one(cx), cx)
    .dual_query_boxed();

  let (position1, position2) = attribute_mesh_access_position_buffer.fork();

  let none_indexed = position1
    .dual_query_filter_by_set(none_indexed_mesh_set)
    .dual_query_boxed();

  let indexed = position2
    .dual_query_intersect(indexed_meshes_and_its_range)
    .dual_query_boxed();

  none_indexed.join(indexed).map(
    |(none_indexed, indexed)| PositionRelatedAttributeMeshQuery {
      none_indexed,
      indexed,
    },
  )
}

#[derive(Clone)]
pub struct PositionRelatedAttributeMeshQuery {
  pub indexed: BoxedDynDualQuery<
    RawEntityHandle, // mesh buffer entity
    (
      (Option<RawEntityHandle>, Option<BufferViewRange>), // position, buffer entity
      (RawEntityHandle, Option<BufferViewRange>, u32), /* index, count(used to distinguish the index format) */
    ),
  >,
  pub none_indexed: BoxedDynDualQuery<
    RawEntityHandle,                                    // mesh buffer entity
    (Option<RawEntityHandle>, Option<BufferViewRange>), // buffer entity
  >,
}
