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
  pub vertices: smallvec::SmallVec<
    [(
      EntityHandle<AttributesMeshEntityVertexBufferRelation>,
      EntityHandle<BufferEntity>,
    ); 3],
  >,
}

impl AttributesMeshEntities {
  /// this method assume the mesh's buffers are owned by mesh it self and not shared
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
    let count = self
      .indices
      .as_ref()
      .map(|(_, data)| data.count as u32)
      .unwrap_or(0);
    let index_data = self.indices.map(|(_, data)| data.write(buffer));

    let index = SceneBufferViewDataView {
      data: index_data,
      range: None,
      count,
    };

    let mesh = writer.mesh.new_entity(|w| {
      let w = w.write::<AttributesMeshEntityTopology>(&self.mode);
      index.write::<AttributeIndexRef>(w)
    });

    let mut vertices = smallvec::SmallVec::with_capacity(self.attributes.len());

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
        count: count as u32,
      };

      let vertex = writer.relation.new_entity(|w| {
        let w = w
          .write::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(
            &mesh.some_handle(),
          )
          .write::<AttributesMeshEntityVertexBufferSemantic>(&semantic);

        vertex.write::<AttributeVertexRef>(w)
      });

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
    cx.use_shared_dual_query(AttributeMeshInput)
      .use_dual_query_execute_map(cx, || {
        |_k, mesh| {
          mesh
            .read_shape()
            .as_abstract_mesh_read_view()
            .primitive_iter()
            .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
        }
      })
  }
}

// fn use_attribute_mesh_local_bounding(
//   cx: &mut impl DBHookCxLike,
//   mesh_inputs: UseResult<Arc<LinearBatchChanges<RawEntityHandle, AttributesMesh>>>,
// ) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3>> {
//   mesh_inputs
//     .map_changes(|mesh| {
//       mesh
//         .read_shape()
//         .as_abstract_mesh_read_view()
//         .primitive_iter()
//         .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
//     })
//     .use_change_to_dual_query_in_spawn_stage(cx)
// }

pub struct AttributeMeshInput;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for AttributeMeshInput {
  type Result =
    impl DualQueryLike<Key = RawEntityHandle, Value = ExternalRefPtr<AttributesMesh>> + 'static; // attribute mesh entity

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    attribute_mesh_input(cx)
      .map_changes(ExternalRefPtr::new)
      .use_change_to_dual_query_in_spawn_stage(cx)
  }
}

pub fn attribute_mesh_input(
  cx: &mut impl DBHookCxLike,
) -> UseResult<Arc<LinearBatchChanges<RawEntityHandle, AttributesMesh>>> {
  let mesh_set_changes = cx.use_query_set::<AttributesMeshEntity>();

  // key: attribute mesh
  // todo, only union key, as we not use value at all
  let index_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeIndexRef>>();
  let index_buffer = index_buffer_ref
    .dual_query_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_boxed();

  // key: middle table
  let vertex_buffer_ref = cx
    .use_dual_query::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .dual_query_boxed();
  let vertex_buffer_range = cx
    .use_dual_query::<SceneBufferViewBufferRange<AttributeVertexRef>>()
    .dual_query_boxed();
  let vertex_buffer_ref_attributes_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_boxed();
  let vertex_buffer = vertex_buffer_ref
    .dual_query_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_boxed();
  let vertex_buffer = vertex_buffer_ref_attributes_mesh
    .dual_query_union(vertex_buffer, |(a, b)| Some((a?, b?)))
    .dual_query_boxed();

  let mesh_changes = mesh_set_changes
    .join(index_buffer.join(vertex_buffer))
    .map_spawn_stage_in_thread(
      cx,
      |(mesh_set_change, (index_buffer, vertex_buffer))| {
        mesh_set_change.has_item_hint()
          || index_buffer.has_delta_hint()
          || vertex_buffer.has_delta_hint()
      },
      |(mesh_set_change, (index_buffer, vertex_buffer))| {
        let mut removed_meshes = FastHashSet::default(); // todo improve
        let mesh_set_change = mesh_set_change.into_change();
        for mesh in mesh_set_change.iter_removed() {
          removed_meshes.insert(mesh);
        }
        let mut re_access_meshes = FastHashSet::default(); // todo, improve capacity
        for (mesh, _) in mesh_set_change.iter_update_or_insert() {
          re_access_meshes.insert(mesh);
        }
        for (mesh, _) in index_buffer.delta.iter_key_value() {
          re_access_meshes.insert(mesh);
        }
        for (_, change) in vertex_buffer.delta.iter_key_value() {
          if let Some((Some(mesh), _)) = change.old_value() {
            re_access_meshes.insert(*mesh);
          }
          if let Some((Some(mesh), _)) = change.new_value() {
            re_access_meshes.insert(*mesh);
          }
        }
        for mesh in &removed_meshes {
          re_access_meshes.remove(mesh);
        }
        (re_access_meshes, removed_meshes)
      },
    );

  let mesh_ref_vertex =
    cx.use_db_rev_ref_tri_view::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>();

  mesh_changes
    .join(mesh_ref_vertex)
    .map_spawn_stage_in_thread(
      cx,
      |(mesh_changes, mesh_ref_vertex)| {
        !mesh_changes.0.is_empty() || !mesh_changes.1.is_empty() || mesh_ref_vertex.has_delta_hint()
      },
      |((re_access_meshes, removed_meshes), mesh_ref_vertex)| {
        let reader = AttributesMeshReader::new_from_global(
          mesh_ref_vertex
            .rev_many_view
            .mark_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
            .into_boxed_multi(),
        );
        let re_access_meshes = re_access_meshes
          .into_iter()
          .map(|m| {
            let mesh = unsafe { EntityHandle::from_raw(m) };
            let mesh = reader.read(mesh).unwrap();
            (m, mesh)
          })
          .collect::<Vec<_>>();

        Arc::new(LinearBatchChanges {
          removed: removed_meshes.into_iter().collect(),
          update_or_insert: re_access_meshes,
        })
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
