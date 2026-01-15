use crate::*;

declare_entity!(AttributesMeshEntity);
declare_component!(
  AttributesMeshEntityTopology,
  AttributesMeshEntity,
  PrimitiveTopology
);

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Facet)]
pub enum BoundingConfig {
  /// the implementation will compute the bounding from it's position and index data.
  ///
  /// **NOTE, if user using uri mesh, and switched to computed later,
  ///  the bounding may not be correctly computed.** It's a valid limitation of current implementation.
  #[default]
  Computed,
  /// using this if:
  /// - the mesh contains skin animation(can not be effectively computed)
  /// - the mesh is uri based and requires bounding info in advance for data scheduling
  /// - the bounding has already been precomputed(for example in gltf import)
  UserDefined(Box3),
}
declare_component!(
  AttributesMeshBoundingConfig,
  AttributesMeshEntity,
  BoundingConfig
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
  pub fn set_user_defined_bounding(
    &mut self,
    mesh: EntityHandle<AttributesMeshEntity>,
    bounding: Box3,
  ) {
    self
      .mesh
      .write::<AttributesMeshBoundingConfig>(mesh, BoundingConfig::UserDefined(bounding));
  }
}

pub trait AttributesMeshWriter {
  fn create_writer() -> AttributesMeshEntityFromAttributesMeshWriter;
  fn write(
    self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    buffer: &mut EntityWriter<BufferEntity>,
  ) -> AttributesMeshEntities;
  fn write_impl(
    self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    buffer: &mut dyn FnMut(AttributeAccessor) -> EntityHandle<BufferEntity>,
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
  /// this method assume the mesh's buffers are owned by mesh itself and not shared
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
    self.write_impl(writer, &mut |data| data.write(buffer))
  }

  fn write_impl(
    self,
    writer: &mut AttributesMeshEntityFromAttributesMeshWriter,
    write_buffer: &mut dyn FnMut(AttributeAccessor) -> EntityHandle<BufferEntity>,
  ) -> AttributesMeshEntities {
    let count = self
      .indices
      .as_ref()
      .map(|(_, data)| data.count as u32)
      .unwrap_or(0);
    let index_data = self.indices.map(|(_, data)| write_buffer(data));

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
      let vertex_data = write_buffer(vertex);

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
    .declare_component::<AttributesMeshEntityTopology>()
    .declare_component::<AttributesMeshBoundingConfig>();

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

  /// todo, output UriLoadResult result
  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let user_defined_bounding = cx
      .use_dual_query::<AttributesMeshBoundingConfig>()
      .dual_query_filter_map(|c| match c {
        BoundingConfig::Computed => None,
        BoundingConfig::UserDefined(aabb) => Some(aabb),
      });

    let computed = (self.0.clone())(cx)
      .map(|mesh| {
        let bounding_config = get_db_view::<AttributesMeshBoundingConfig>();
        mesh.collective_filter_kv_map(move |k, mesh| {
          if let UriLoadResult::LivingOrLoaded(mesh) = mesh {
            if let BoundingConfig::Computed = bounding_config.read_ref(*k).unwrap() {
              let mesh = mesh.into_attributes_mesh();
              let position = mesh.get_position_slice();
              Some(
                mesh
                  .create_abstract_mesh_view(position)
                  .primitive_iter()
                  .fold(Box3::empty(), |b, p| b.union_into(p.to_bounding())),
              )
            } else {
              None
            }
          } else {
            None
          }
        })
      })
      .use_change_to_dual_query_in_spawn_stage(cx);

    computed.dual_query_select(user_defined_bounding)
  }
}

pub type AttributeVertexDataSource =
  UseResult<Arc<LinearBatchChanges<RawEntityHandle, AttributeLivingData>>>;

pub type AttributeIndexDataSource =
  UseResult<Arc<LinearBatchChanges<RawEntityHandle, AttributeLivingData>>>;

/// the output changes are assumed to be consumed by gpu systems.
/// the current implementation is not considering the buffer share between the difference views.
/// this can be improved(not easy to do so) but not necessary for now.
///
/// todo, output UriLoadResult result
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
      if let UriLoadResult::LivingOrLoaded(mesh_info) = mesh_info {
        if let Some(indices) = &mesh_info.indices {
          indices_changes
            .update_or_insert
            .push((mesh, indices.clone()));
        }

        let mut vertices_record = Vec::new();
        for v in mesh_info.vertices {
          vertices_changes
            .update_or_insert
            .push((v.relation_handle, v.data.clone()));
          vertices_record.push(v.relation_handle)
        }
        vertex_mapping.insert(mesh, vertices_record);
      }
    }

    (Arc::new(indices_changes), Arc::new(vertices_changes))
  });

  let (indices_changes, vertices_changes) = changes.fork();

  let indices_changes = indices_changes.map(|(i, _)| i);
  let vertices_changes = vertices_changes.map(|(_, v)| v);

  (vertices_changes, indices_changes)
}

pub type AttributesMeshDataChangeInput =
  Arc<LinearBatchChanges<RawEntityHandle, UriLoadResult<AttributesMeshWithVertexRelationInfo>>>;
