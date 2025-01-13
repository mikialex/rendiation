use std::{mem::offset_of, num::NonZeroU64, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::{AttributeSemantic, BufferViewRange};
use rendiation_shader_api::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

pub fn attribute_indices(
  index_pool: &UntypedPool,
  gpu: &GPU,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = Vec2<u32>> {
  let index_buffer_ref =
    global_watch().watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  // todo, put it into registry
  let source = index_buffer_ref
    .collective_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .collective_zip(global_watch().watch::<SceneBufferViewBufferItemCount<AttributeIndexRef>>())
    .collective_filter_map(|((index, range), count)| index.map(|i| (i, range, count.unwrap())))
    .collective_execute_map_by(|| {
      let data = global_entity_component_of::<BufferEntityData>().read();
      move |_, (buffer_id, range, count)| {
        let count = count as usize;
        let buffer = data.get(buffer_id).unwrap().ptr.clone();
        if buffer.len() / count == 4 {
          (buffer, range_convert(range))
        } else if buffer.len() / count == 2 {
          let buffer = bytemuck::cast_slice::<_, u16>(&buffer);
          let buffer = buffer.iter().map(|i| *i as u32).collect::<Vec<_>>();
          let buffer = bytemuck::cast_slice::<_, u8>(buffer.as_slice());
          let buffer = Arc::new(buffer.to_vec());
          (buffer, None)
        } else {
          unreachable!("index count must be 2 or 4")
        }
      }
    })
    .into_boxed();

  ReactiveRangeAllocatePool::new(index_pool, source, gpu)
    .collective_map(|(offset, count)| Vec2::new(offset, count))
}

fn range_convert(range: Option<BufferViewRange>) -> Option<GPUBufferViewRange> {
  range.map(|r| GPUBufferViewRange {
    offset: r.offset,
    size: r.size,
  })
}

pub fn attribute_vertex(
  pool: &UntypedPool,
  semantic: AttributeSemantic,
  gpu: &GPU,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = u32> {
  let attribute_scope = global_watch()
    .watch::<AttributesMeshEntityVertexBufferSemantic>()
    .collective_filter(move |s| semantic == s)
    .collective_map(|_| {})
    .into_forker();

  let vertex_buffer_ref = global_watch()
    .watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .filter_by_keyset(attribute_scope.clone());

  let vertex_buffer_range = global_watch()
    .watch::<SceneBufferViewBufferRange<AttributeVertexRef>>()
    .filter_by_keyset(attribute_scope.clone());

  let ranged_buffer = vertex_buffer_ref
    .collective_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .collective_filter_map(|(index, range)| index.map(|i| (i, range)))
    .collective_execute_map_by(|| {
      let data = global_entity_component_of::<BufferEntityData>().read();
      move |_, v| (data.get(v.0).unwrap().ptr.clone(), range_convert(v.1))
    })
    .into_boxed();

  let ab_ref_mesh = global_watch()
    .watch_typed_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .collective_filter_map(|v| v)
    .filter_by_keyset(attribute_scope)
    .hash_reverse_assume_one_one();

  // we not using intersect here because range may not exist
  // todo, put it into registry
  ReactiveRangeAllocatePool::new(pool, ranged_buffer, gpu)
    .collective_map(|(offset, _)| offset)
    .one_to_many_fanout(ab_ref_mesh.into_one_to_many_by_hash())
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, PartialEq, Copy, ShaderStruct, Default, StorageNodePtrAccess)]
pub struct AttributeMeshMeta {
  pub index_offset: u32,
  pub count: u32,
  pub position_offset: u32,
  pub normal_offset: u32,
  pub uv_offset: u32,
}

pub type CommonStorageBufferImplWithHostBackup<T> =
  VecWithStorageBuffer<GrowableDirectQueueUpdateBuffer<StorageBufferReadOnlyDataView<[T]>>>;

pub fn attribute_buffer_metadata(
  gpu: &GPU,
  index_pool: &UntypedPool,
  position_pool: &UntypedPool,
  normal_pool: &UntypedPool,
  uv_pool: &UntypedPool,
) -> MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>> {
  let base = ReactiveStorageBufferContainer::<AttributeMeshMeta>::new(gpu);
  let new_base = base
    .inner
    .target
    .with_vec_backup(AttributeMeshMeta::default(), false);
  let data = MultiUpdateContainer::new(new_base);

  data
    .with_source(QueryBasedStorageBufferUpdate {
      // note, the offset and count is update together
      field_offset: offset_of!(AttributeMeshMeta, index_offset) as u32,
      upstream: attribute_indices(index_pool, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, position_offset) as u32,
      upstream: attribute_vertex(position_pool, AttributeSemantic::Positions, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, normal_offset) as u32,
      upstream: attribute_vertex(normal_pool, AttributeSemantic::Normals, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, uv_offset) as u32,
      upstream: attribute_vertex(uv_pool, AttributeSemantic::TexCoords(0), gpu),
    })
}

pub struct MeshBindlessGPUSystemSource {
  attribute_buffer_metadata: UpdateResultToken,
  sm_to_mesh: UpdateResultToken,
  sm_to_mesh_device: UpdateResultToken,
  indices: UntypedPool,
  position: UntypedPool, // using untyped to avoid padding waste
  normal: UntypedPool,
  uv: UntypedPool,
}

impl MeshBindlessGPUSystemSource {
  pub fn new(gpu: &GPU) -> Self {
    let indices_init_size = 20 * 1024 * 1024;
    let indices_max_size = 200 * 1024 * 1024;

    let indices = StorageBufferReadOnlyDataView::<[u32]>::create_by_with_extra_usage(
      &gpu.device,
      StorageBufferInit::Zeroed(NonZeroU64::new(indices_init_size as u64).unwrap()),
      BufferUsages::INDEX,
    );

    let indices = create_growable_buffer(gpu, indices, indices_max_size);
    let indices = GPURangeAllocateMaintainer::new(gpu, indices);

    let position =
      create_storage_buffer_range_allocate_pool(gpu, 100 * 1024 * 1024, 1000 * 1024 * 1024);
    let normal =
      create_storage_buffer_range_allocate_pool(gpu, 100 * 1024 * 1024, 1000 * 1024 * 1024);
    let uv = create_storage_buffer_range_allocate_pool(gpu, 80 * 1024 * 1024, 800 * 1024 * 1024);

    Self {
      attribute_buffer_metadata: Default::default(),
      sm_to_mesh: Default::default(),
      sm_to_mesh_device: Default::default(),
      indices: Arc::new(RwLock::new(indices)),
      position: Arc::new(RwLock::new(position)),
      normal: Arc::new(RwLock::new(normal)),
      uv: Arc::new(RwLock::new(uv)),
    }
  }

  pub fn create_impl_internal_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> MeshGPUBindlessImpl {
    let vertex_address_buffer = res
      .take_multi_updater_updated::<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>(
        self.attribute_buffer_metadata,
      )
      .unwrap();

    MeshGPUBindlessImpl {
      indices: self.indices.clone(),
      position: self.position.clone(),
      normal: self.normal.clone(),
      uv: self.uv.clone(),
      checker: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      indices_checker: global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
        .read_foreign_key(),
      vertex_address_buffer: vertex_address_buffer.gpu().clone().into_rw_view(),
      vertex_address_buffer_host: vertex_address_buffer.clone(),
      sm_to_mesh_device: res
        .take_multi_updater_updated::<CommonStorageBufferImpl<u32>>(self.sm_to_mesh_device)
        .unwrap()
        .gpu()
        .clone()
        .into_rw_view(),
      sm_to_mesh: res.take_reactive_query_updated(self.sm_to_mesh).unwrap(),
    }
  }
}

impl RenderImplProvider<Box<dyn IndirectModelShapeRenderImpl>> for MeshBindlessGPUSystemSource {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.attribute_buffer_metadata = source.register_multi_updater(attribute_buffer_metadata(
      cx,
      &self.indices,
      &self.position,
      &self.normal,
      &self.uv,
    ));

    let sm_to_mesh = global_watch()
      .watch_typed_foreign_key::<StandardModelRefAttributesMeshEntity>()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
      .into_forker();

    let sm_to_mesh_device_source = sm_to_mesh
      .clone()
      .collective_map(|v| v.map(|v| v.alloc_index()).unwrap_or(u32::MAX));

    let sm_to_mesh_device =
      ReactiveStorageBufferContainer::<u32>::new(cx).with_source(sm_to_mesh_device_source, 0);

    self.sm_to_mesh_device = source.register_multi_updater(sm_to_mesh_device.inner);
    let sm_to_mesh = sm_to_mesh.collective_filter_map(|v| v);
    self.sm_to_mesh = source.register_reactive_query(sm_to_mesh);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.attribute_buffer_metadata);
    source.deregister(&mut self.sm_to_mesh_device);
    source.deregister(&mut self.sm_to_mesh);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn IndirectModelShapeRenderImpl> {
    Box::new(self.create_impl_internal_impl(res))
  }
}

pub struct MeshGPUBindlessImpl {
  indices: UntypedPool,
  position: UntypedPool,
  normal: UntypedPool,
  uv: UntypedPool,
  vertex_address_buffer: StorageBufferDataView<[AttributeMeshMeta]>,
  vertex_address_buffer_host: LockReadGuardHolder<
    MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
  >,
  sm_to_mesh_device: StorageBufferDataView<[u32]>,
  sm_to_mesh: BoxedDynQuery<EntityHandle<SceneModelEntity>, EntityHandle<AttributesMeshEntity>>,
  checker: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  indices_checker: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
}

impl MeshGPUBindlessImpl {
  pub fn make_bindless_dispatcher(&self) -> BindlessMeshDispatcher {
    let position =
      StorageBufferDataView::try_from_raw(self.position.read().raw_gpu().clone()).unwrap();
    let normal = StorageBufferDataView::try_from_raw(self.normal.read().raw_gpu().clone()).unwrap();
    let uv = StorageBufferDataView::try_from_raw(self.uv.read().raw_gpu().clone()).unwrap();

    let index_pool =
      StorageBufferDataView::try_from_raw(self.indices.read().raw_gpu().clone()).unwrap();

    BindlessMeshDispatcher {
      sm_to_mesh: self.sm_to_mesh_device.clone(),
      vertex_address_buffer: self.vertex_address_buffer.clone(),
      position,
      normal,
      uv,
      index_pool,
    }
  }
}

impl IndirectModelShapeRenderImpl for MeshGPUBindlessImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    // check the given model has attributes mesh
    let mesh_id = self.checker.get(any_idx)?;
    // check mesh must have indices.
    let _ = self.indices_checker.get(mesh_id)?;
    Some(Box::new(self.make_bindless_dispatcher()))
  }

  fn hash_shader_group_key(
    &self,
    _any_id: EntityHandle<StandardModelEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    // check the given model has attributes mesh
    let mesh_id = self.checker.get(any_idx)?;
    // check mesh must have indices.
    let _ = self.indices_checker.get(mesh_id)?;
    Some(Box::new(BindlessDrawCreator {
      metadata: self.vertex_address_buffer.clone(),
      sm_to_mesh_device: self.sm_to_mesh_device.clone(),
      sm_to_mesh: self.sm_to_mesh.clone(),
      vertex_address_buffer_host: self.vertex_address_buffer_host.clone(),
    }))
  }
}

#[derive(Clone)]
pub struct BindlessMeshDispatcher {
  // todo, use readonly
  pub sm_to_mesh: StorageBufferDataView<[u32]>,
  pub vertex_address_buffer: StorageBufferDataView<[AttributeMeshMeta]>,
  pub index_pool: StorageBufferDataView<[u32]>,
  pub position: StorageBufferDataView<[u32]>,
  pub normal: StorageBufferDataView<[u32]>,
  pub uv: StorageBufferDataView<[u32]>,
}

impl ShaderHashProvider for BindlessMeshDispatcher {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for BindlessMeshDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.vertex_address_buffer);

    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.index_pool, IndexFormat::Uint32);

    ctx.binding.bind(&self.position);
    ctx.binding.bind(&self.normal);
    ctx.binding.bind(&self.uv);
  }
}

impl GraphicsShaderProvider for BindlessMeshDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let mesh_handle = vertex.query::<IndirectAbstractMeshId>();
      let vertex_id = vertex.query::<VertexIndex>();

      let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
      let vertex_address = vertex_addresses.index(mesh_handle).load().expand();

      let position = binding.bind_by(&self.position);
      let normal = binding.bind_by(&self.normal);
      let uv = binding.bind_by(&self.uv);
      unsafe {
        let position = Vec3::<f32>::sized_ty()
          .load_from_u32_buffer(
            position,
            vertex_address.position_offset + vertex_id * val(3),
          )
          .cast_type::<Vec3<f32>>();

        let normal = Vec3::<f32>::sized_ty()
          .load_from_u32_buffer(normal, vertex_address.normal_offset + vertex_id * val(3))
          .cast_type::<Vec3<f32>>();

        let uv = Vec2::<f32>::sized_ty()
          .load_from_u32_buffer(uv, vertex_address.uv_offset + vertex_id * val(2))
          .cast_type::<Vec2<f32>>();

        vertex.register::<GeometryPosition>(position);
        vertex.register::<GeometryNormal>(normal);
        vertex.register::<GeometryUV>(uv);
      }
    })
  }
}

#[derive(Clone)]
pub struct BindlessDrawCreator {
  metadata: StorageBufferDataView<[AttributeMeshMeta]>,
  sm_to_mesh: BoxedDynQuery<EntityHandle<SceneModelEntity>, EntityHandle<AttributesMeshEntity>>,
  sm_to_mesh_device: StorageBufferDataView<[u32]>,
  vertex_address_buffer_host: LockReadGuardHolder<
    MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
  >,
}

impl DrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .vec
      .get(mesh_id.alloc_index() as usize)
      .unwrap();

    let start = address_info.index_offset;
    let end = start + address_info.count;
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: start..end,
      instances: 0..1,
    }
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DrawCommandBuilderInvocation> {
    let node = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(BindlessDrawCreatorInDevice {
      node,
      sm_to_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl ShaderPassBuilder for BindlessDrawCreator {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    cx.binding.bind(&self.metadata);
  }
}

impl ShaderHashProvider for BindlessDrawCreator {
  shader_hash_type_id! {}
}

pub struct BindlessDrawCreatorInDevice {
  node: StorageNode<[AttributeMeshMeta]>,
  sm_to_mesh_device: StorageNode<[u32]>,
}

impl DrawCommandBuilderInvocation for BindlessDrawCreatorInDevice {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // todo check mesh_handle

    let meta = self.node.index(mesh_handle).load().expand();
    ENode::<DrawIndexedIndirect> {
      vertex_count: meta.count,
      instance_count: val(1),
      base_index: meta.index_offset,
      vertex_offset: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
