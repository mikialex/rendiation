use fast_hash_collection::FastHashSet;
use rendiation_uri_streaming::*;

use crate::*;

pub type ViewerTextureDataSource = dyn UriDataSourceDyn<Arc<GPUBufferImage>>;
pub type ViewerMeshDataSource = dyn UriDataSourceDyn<Arc<Vec<u8>>>;

type TextureScheduler = NoControlStreaming<u32, Arc<GPUBufferImage>, Arc<String>>;
type MeshScheduler =
  NoControlStreaming<RawEntityHandle, AttributesMeshWithVertexRelationInfo, AttributesMeshWithUri>;

pub struct ViewerDataScheduler {
  pub texture_uri_backend: Arc<RwLock<Box<ViewerTextureDataSource>>>,
  texture: Arc<RwLock<TextureScheduler>>,
  pub mesh_uri_backend: Arc<RwLock<Box<ViewerMeshDataSource>>>,
  mesh: Arc<RwLock<MeshScheduler>>,
}

impl Default for ViewerDataScheduler {
  fn default() -> Self {
    #[cfg(not(target_family = "wasm"))]
    let texture_uri_backend = {
      let exe_path = std::env::current_exe().unwrap();
      let root = exe_path.parent().unwrap().join("temp_textures/");
      if root.is_dir() {
        std::fs::remove_dir_all(&root).unwrap(); // clean up old, if last run not exist normally
      }

      let texture_uri_backend = URIDiskSyncSource::<Arc<GPUBufferImage>>::new(
        root,
        |image| {
          // consider do encoding png for common fmt?
          rmp_serde::to_vec(image.as_ref()).unwrap()
        },
        |bytes| {
          let image: GPUBufferImage = rmp_serde::from_slice(bytes).unwrap();
          Arc::new(image)
        },
      );
      Box::new(texture_uri_backend) as Box<dyn UriDataSourceDyn<_>>
    };

    #[cfg(target_family = "wasm")]
    let texture_uri_backend = {
      let texture_uri_backend =
        InMemoryUriDataSource::<Arc<GPUBufferImage>>::new(alloc_global_res_id());
      Box::new(texture_uri_backend) as Box<dyn UriDataSourceDyn<_>>
    };

    let texture_uri_backend = Arc::new(RwLock::new(texture_uri_backend));

    let scheduler = NoControlStreaming::default();
    let texture = Arc::new(RwLock::new(scheduler));

    let mesh_buffer_uri_backend = InMemoryUriDataSource::<Arc<Vec<u8>>>::new(alloc_global_res_id());
    let mesh_buffer_uri_backend = Box::new(mesh_buffer_uri_backend) as Box<dyn UriDataSourceDyn<_>>;
    let mesh_buffer_uri_backend = Arc::new(RwLock::new(mesh_buffer_uri_backend));

    let scheduler = NoControlStreaming::default();
    let mesh = Arc::new(RwLock::new(scheduler));

    Self {
      texture,
      mesh,
      texture_uri_backend,
      mesh_uri_backend: mesh_buffer_uri_backend,
    }
  }
}

pub fn viewer_mesh_input<Cx>(cx: &mut Cx) -> UseResult<AttributesMeshDataChangeInput>
where
  Cx: DBHookCxLike,
{
  struct DBMeshInput;
  impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBMeshInput {
    share_provider_hash_type_id! {}
    type Result = MeshInput;
    fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
      attribute_mesh_input(cx)
    }
  }

  access_cx!(cx.dyn_env(), s, ViewerDataScheduler);
  let scheduler = s.mesh.clone();
  let source = s.mesh_uri_backend.clone();
  let loader_creator = move || {
    let mut source = source.make_write_holder();
    Box::new(move |uri: &AttributesMeshWithUri| {
      let source = &mut *source;
      load_uri_mesh(uri, source.as_mut())
        as Box<
          dyn Future<Output = Option<AttributesMeshWithVertexRelationInfo>> + Send + Sync + Unpin,
        >
    }) as Box<LoaderFunction<AttributesMeshWithUri, AttributesMeshWithVertexRelationInfo>>
  };
  let loader_creator = Arc::new(loader_creator) as Arc<_>;

  use_uri_data_changes(cx, DBMeshInput, &scheduler, loader_creator)
}

pub fn viewer_mesh_buffer_input(
  cx: &mut QueryGPUHookCx<'_>,
) -> (AttributeVertexDataSource, AttributeIndexDataSource) {
  let mesh_changes = viewer_mesh_input(cx);
  create_sub_buffer_changes_from_mesh_changes(cx, mesh_changes)
}

fn load_uri_mesh(
  mesh: &AttributesMeshWithUri,
  buffer_backend: &mut dyn UriDataSourceDyn<Arc<Vec<u8>>>,
) -> Box<
  dyn Future<Output = Option<AttributesMeshWithVertexRelationInfo>> + Send + Sync + Unpin + 'static,
> {
  fn create_buffer_fut(
    data: &AttributeUriData,
    buffer_backend: &mut dyn UriDataSourceDyn<Arc<Vec<u8>>>,
  ) -> Pin<Box<dyn Future<Output = Option<AttributeLivingData>> + Send + Sync + 'static>> {
    let fut = match &data.data {
      MaybeUriData::Uri(uri) => Some(buffer_backend.request_uri_data_load_dyn(uri.as_str())),
      MaybeUriData::Living(_) => None,
    };

    let range = data.range;
    let count = data.count;
    let living_data = data.data.clone().into_living();

    Box::pin(async move {
      let buffer = if let Some(fut) = fut {
        fut.await
      } else {
        Some(living_data.unwrap())
      };

      buffer.map(|b| AttributeLivingData {
        data: b,
        range,
        count,
      })
    })
  }

  fn create_vertex_buffer_fut(
    se: AttributeSemantic,
    handle: RawEntityHandle,
    data: &AttributeUriData,
    buffer_backend: &mut dyn UriDataSourceDyn<Arc<Vec<u8>>>,
  ) -> Pin<Box<dyn Future<Output = Option<AttributeMeshLivingVertex>> + Send + Sync + 'static>> {
    let fut = create_buffer_fut(data, buffer_backend).map(move |r| {
      r.map(|data| AttributeMeshLivingVertex {
        semantic: se,
        relation_handle: handle,
        data,
      })
    });

    Box::pin(fut)
  }

  let indices = mesh
    .indices
    .as_ref()
    .map(|indices| create_buffer_fut(indices, buffer_backend));

  let vertices: Vec<_> = mesh
    .vertices
    .iter()
    .map(|vertices| {
      create_vertex_buffer_fut(
        vertices.semantic.clone(),
        vertices.relation_handle,
        &vertices.data,
        buffer_backend,
      )
    })
    .collect();

  let mode = mesh.mode;

  Box::new(Box::pin(async move {
    let indices = if let Some(indices) = indices {
      let indices = indices.await;
      if let Some(indices) = indices {
        Some(indices)
      } else {
        return None;
      }
    } else {
      None
    };

    let vertices = futures::future::join_all(vertices).await;
    let len = vertices.len();
    let vertices: Vec<_> = vertices.into_iter().flatten().collect();

    if len != vertices.len() {
      return None;
    }

    AttributesMeshWithVertexRelationInfo {
      mode,
      indices,
      vertices,
    }
    .into()
  }))
}

type MeshInput = DataChangesAndLivingReInit<
  RawEntityHandle,
  AttributesMeshWithVertexRelationInfo,
  AttributesMeshWithUri,
>;

pub fn attribute_mesh_input(cx: &mut impl DBHookCxLike) -> UseResult<MeshInput> {
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
            let mesh = reader.read(mesh).unwrap().into_maybe_uri_form();
            (m, mesh)
          })
          .collect::<Vec<_>>();

        let changes = Arc::new(LinearBatchChanges {
          removed: removed_meshes.into_iter().collect(),
          update_or_insert: re_access_meshes,
        });

        let set = get_db_set_view::<AttributesMeshEntity>();
        let iter = MeshInputIter(reader, set);

        DataChangesAndLivingReInit {
          changes,
          iter_living_full: Arc::new(iter),
        }
      },
    )
}

struct MeshInputIter(AttributesMeshReader, BoxedDynQuery<RawEntityHandle, ()>);

impl LivingDataReInitIteratorProvider for MeshInputIter {
  type Item = (RawEntityHandle, AttributesMeshWithVertexRelationInfo);

  fn create_iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_> {
    let iter = self.1.iter_key_value().filter_map(|(rk, _)| {
      let k = unsafe { EntityHandle::from_raw(rk) };
      match self.0.read(k).unwrap().into_maybe_uri_form() {
        MaybeUriData::Uri(_) => None,
        MaybeUriData::Living(v) => Some((rk, v)),
      }
    });
    Box::new(iter)
  }
}

// todo, LinearBatchChanges<u32, Option<GPUBufferImage>>'s iter will cause excessive clone
// so we use Arc, but we should use DataChangeRef trait
pub fn viewer_texture_input(
  cx: &mut QueryGPUHookCx<'_>,
) -> UseResult<Arc<LinearBatchChanges<u32, UriLoadResult<Arc<GPUBufferImage>>>>> {
  struct TextureIter(DBViewUnchecked<TextureDirectContentType>);
  impl LivingDataReInitIteratorProvider for TextureIter {
    type Item = (u32, Arc<GPUBufferImage>);

    fn create_iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_> {
      Box::new(self.0.iter_key_value().filter_map(|(k, v)| {
        let v = v?;
        let v = match v.ptr.as_ref() {
          MaybeUriData::Uri(_) => None,
          MaybeUriData::Living(v) => Some(v),
        }?;
        Some((k, v.clone()))
      }))
    }
  }

  struct DBTextureUriInput;
  impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBTextureUriInput {
    share_provider_hash_type_id! {}
    type Result = DataChangesAndLivingReInit<u32, Arc<GPUBufferImage>, Arc<String>>;
    fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
      cx.use_changes_internal::<<SceneTexture2dEntityDirectContent as ComponentSemantic>::Data>(
        SceneTexture2dEntityDirectContent::component_id(),
        <SceneTexture2dEntityDirectContent as EntityAssociateSemantic>::Entity::entity_id(),
        true,
      )
      .map_spawn_stage_in_thread_data_changes(cx, |changes| {
        let changes = changes
          .collective_filter_map(|v| v.map(|v| (*v).clone()))
          .materialize();

        DataChangesAndLivingReInit {
          changes,
          iter_living_full: Arc::new(TextureIter(get_db_view_no_generation_check::<
            SceneTexture2dEntityDirectContent,
          >())),
        }
      })
    }
  }

  access_cx!(cx.dyn_env(), s, ViewerDataScheduler);
  let scheduler = s.texture.clone();
  let source = s.texture_uri_backend.clone();

  let loader_creator = move || {
    let mut source = source.make_write_holder();
    Box::new(move |uri: &Arc<String>| {
      let source = &mut *source;
      Box::new(source.request_uri_data_load_dyn(uri.as_str()))
        as Box<dyn Future<Output = Option<Arc<GPUBufferImage>>> + Send + Sync + Unpin>
    }) as Box<LoaderFunction<Arc<String>, Arc<GPUBufferImage>>>
  };
  let loader_creator = Arc::new(loader_creator) as Arc<_>;

  use_uri_data_changes(cx, DBTextureUriInput, &scheduler, loader_creator)
}
