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
  let table = global_database()
    .declare_entity::<AttributesMeshEntity>()
    .declare_component::<AttributesMeshEntityTopology>();

  register_scene_buffer_view::<AttributeIndexRef>(table);

  global_database()
    .declare_entity::<BufferEntity>()
    .declare_component::<BufferEntityData>();

  let table = global_database()
    .declare_entity::<AttributesMeshEntityVertexBufferRelation>()
    .declare_component::<AttributesMeshEntityVertexBufferSemantic>()
    .declare_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>();

  register_scene_buffer_view::<AttributeVertexRef>(table);
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

pub struct AttributeMeshLocalBounding<T>(pub T);

impl<Cx, T> SharedResultProvider<Cx> for AttributeMeshLocalBounding<T>
where
  Cx: DBHookCxLike,
  T: FnOnce(&mut Cx) -> UseResult<AttributesMeshDataChangeInput> + Clone,
{
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3> + 'static; // attribute mesh entity
  share_provider_hash_type_id! {AttributeMeshLocalBounding<()>}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    (self.0.clone())(cx)
      .map_changes(|mesh| {
        if let Some(mesh) = mesh {
          let position = mesh.get_position_slice();
          mesh
            .create_abstract_mesh_view(position)
            .primitive_iter()
            .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding()))
        } else {
          Box3::empty()
        }
      })
      .use_change_to_dual_query_in_spawn_stage(cx)
  }
}

pub type AttributeVertexDataSource =
  UseResult<Arc<LinearBatchChanges<RawEntityHandle, (RawEntityHandle, Option<BufferViewRange>)>>>;

// #[define_opaque(AttributeVertexDataSource)]
// pub fn use_attribute_vertex_data(cx: &mut impl DBHookCxLike) -> AttributeVertexDataSource {
//   let vertex_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeVertexRef>>();
//   let vertex_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeVertexRef>>();

//   vertex_buffer_ref
//     .dual_query_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
//     .dual_query_filter_map(|(index, range)| index.map(|i| (i, range)))
//     .dual_query_boxed()
//     .into_delta_change()
// }

pub type AttributeIndexDataSource = UseResult<
  Arc<LinearBatchChanges<RawEntityHandle, (RawEntityHandle, Option<BufferViewRange>, u32)>>,
>;

// #[define_opaque(AttributeIndexDataSource)]
// pub fn use_attribute_index_data(cx: &mut impl DBHookCxLike) -> AttributeIndexDataSource {
//   let index_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>();
//   let index_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeIndexRef>>();
//   // we need count to distinguish between the u16 or u32 index
//   let index_item_count = cx.use_dual_query::<SceneBufferViewBufferItemCount<AttributeIndexRef>>();

//   index_buffer_ref
//     .dual_query_union(index_buffer_range, |(a, b)| Some((a?, b?)))
//     .dual_query_zip(index_item_count)
//     .dual_query_filter_map(|((index, range), count)| index.map(|i| (i, range, count)))
//     .dual_query_boxed()
//     .into_delta_change()
// }

#[derive(Clone)]
pub struct MaybeUriMesh {}

fn read_maybe_uri_mesh(
  reader: &AttributesMeshReader,
) -> MaybeUriData<AttributesMesh, MaybeUriMesh> {
  todo!()
}

/// the output changes are assumed to be consumed by gpu systems.
/// the current implementation is not considering the buffer share between the difference views.
/// this can be improved(not easy to do so) but not necessary for now.
pub fn create_sub_buffer_changes_from_mesh_changes(
  cx: &mut impl DBHookCxLike,
  mesh_changes: UseResult<AttributesMeshDataChangeInput>,
) -> (AttributeVertexDataSource, AttributeIndexDataSource) {
  let vertex_mapping = cx.use_shared_hash_map::<RawEntityHandle, Vec<RawEntityHandle>>(
    "vertex_mapping for mesh buffer change conversion",
  );
  let changes = mesh_changes.map_spawn_stage_in_thread_data_changes(cx, move |mesh_changes| {
    let mut indices_changes = LinearBatchChanges::default();
    let mut vertices_changes = LinearBatchChanges::default();

    // even if some mesh does not have index, we can still put it in removed indices
    // because the data changes allow us to remove none exist item
    indices_changes.removed = mesh_changes.removed.clone();

    // generate removed vertex from recorded vertex mapping info
    let mut vertex_mapping = vertex_mapping.write();
    for removed_mesh in &mesh_changes.removed {
      let vertex = vertex_mapping.remove(removed_mesh).unwrap();
      vertices_changes.removed.extend(vertex);
    }

    for (mesh, mesh_info) in mesh_changes.iter_update_or_insert() {
      //
    }

    (Arc::new(indices_changes), Arc::new(vertices_changes))
  });

  let (indices_changes, vertices_changes) = changes.fork();

  let indices_changes = indices_changes.map(|(i, _)| i);
  let vertices_changes = vertices_changes.map(|(_, v)| v);

  (vertices_changes, indices_changes)
}

pub type AttributesMeshDataChangeInput =
  Arc<LinearBatchChanges<RawEntityHandle, Option<AttributesMesh>>>;

pub type AttributesMeshDataChangeMaybeUriInput =
  Arc<LinearBatchChanges<RawEntityHandle, MaybeUriData<AttributesMesh, MaybeUriMesh>>>;

pub fn attribute_mesh_input(
  cx: &mut impl DBHookCxLike,
) -> UseResult<AttributesMeshDataChangeMaybeUriInput> {
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
            let mesh = MaybeUriData::Living(reader.read(mesh).unwrap());
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
