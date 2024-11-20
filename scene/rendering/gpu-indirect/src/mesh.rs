use std::{mem::offset_of, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::{AttributeSemantic, BufferViewRange};
use rendiation_shader_api::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

#[derive(Clone)]
pub struct IndexPool {
  buffer: Arc<RwLock<RangeAllocatePool<TypedGPUBuffer<u32>>>>,
}

#[derive(Clone)]
pub struct VertexPool {
  buffer: Arc<RwLock<RangeAllocatePool<TypedGPUBuffer<u32>>>>,
}

pub fn attribute_indices(
  index_pool: &IndexPool,
  gpu: &GPU,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = u32> {
  let index_buffer_ref =
    global_watch().watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  // todo, put it into registry
  let source = index_buffer_ref
    .collective_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .collective_filter_map(|(index, range)| index.map(|i| (i, range)))
    .collective_execute_map_by(|| {
      let data = global_entity_component_of::<BufferEntityData>().read();
      move |_, v| (data.get(v.0).unwrap().ptr.clone(), range_convert(v.1))
    })
    .into_boxed();

  ReactiveRangeAllocatePool::new(index_pool.buffer.clone(), source, gpu)
}

fn range_convert(range: Option<BufferViewRange>) -> Option<GPUBufferViewRange> {
  range.map(|r| GPUBufferViewRange {
    offset: r.offset,
    size: r.size,
  })
}

pub fn attribute_vertex(
  pool: &VertexPool,
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
  ReactiveRangeAllocatePool::new(pool.buffer.clone(), ranged_buffer, gpu)
    .one_to_many_fanout(ab_ref_mesh.into_one_to_many_by_hash())
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, PartialEq, Copy, ShaderStruct)]
pub struct AttributeMeshMeta {
  pub index_offset: u32,
  pub count: u32,
  pub position_offset: u32,
  pub normal_offset: u32,
  pub uv_offset: u32,
}

pub fn attribute_buffer_metadata(
  gpu: &GPU,
  index_pool: &IndexPool,
  vertex_pool: &VertexPool,
) -> ReactiveStorageBufferContainer<AttributeMeshMeta> {
  ReactiveStorageBufferContainer::<AttributeMeshMeta>::new(gpu)
    // todo count
    .with_source(
      attribute_indices(index_pool, gpu),
      offset_of!(AttributeMeshMeta, index_offset),
    )
    .with_source(
      attribute_vertex(vertex_pool, AttributeSemantic::Positions, gpu),
      offset_of!(AttributeMeshMeta, position_offset),
    )
    .with_source(
      attribute_vertex(vertex_pool, AttributeSemantic::Normals, gpu),
      offset_of!(AttributeMeshMeta, normal_offset),
    )
    .with_source(
      attribute_vertex(vertex_pool, AttributeSemantic::TexCoords(0), gpu),
      offset_of!(AttributeMeshMeta, uv_offset),
    )
}

pub struct MeshBindlessGPUSystemSource {
  attribute_buffer_metadata: UpdateResultToken,
  indices: IndexPool,
  vertex: VertexPool,
}

impl MeshBindlessGPUSystemSource {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      attribute_buffer_metadata: Default::default(),
      indices: todo!(),
      vertex: todo!(),
    }
  }
}

impl RenderImplProvider<Box<dyn IndirectModelShapeRenderImpl>> for MeshBindlessGPUSystemSource {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let index_pool = todo!();
    let vertex_pool = todo!();
    self.attribute_buffer_metadata =
      source.register_multi_updater(attribute_buffer_metadata(cx, &index_pool, &vertex_pool).inner);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.attribute_buffer_metadata);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn IndirectModelShapeRenderImpl> {
    Box::new(MeshGPUBindlessImpl {
      indices: self.indices.clone(),
      vertex: self.vertex.clone(),
      checker: todo!(),
    })
  }
}

struct MeshGPUBindlessImpl {
  indices: IndexPool,
  vertex: VertexPool,
  //   source: BoxedDynReactiveQuery<EntityHandle<StandardModelEntity>, MeshSystemMeshInstance>,
  checker: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
}

impl IndirectModelShapeRenderImpl for MeshGPUBindlessImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let _ = self.checker.get(any_idx)?; // check the given model has attributes mesh
                                        // todo, check mesh must have indices.
    Some(Box::new(BindlessMeshDispatcher {
      vertex_address_buffer: todo!(),
      vertex_pool: todo!(),
      index_pool: todo!(),
    }))
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    todo!()
  }
}

pub struct BindlessMeshDispatcher {
  // todo, use readonly
  vertex_address_buffer: StorageBufferDataView<[AttributeMeshMeta]>,
  index_pool: GPUBufferResourceView,
  vertex_pool: StorageBufferDataView<[u32]>,
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

    ctx.binding.bind(&self.vertex_pool);
  }
}

impl GraphicsShaderProvider for BindlessMeshDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let mesh_handle = vertex.query::<IndirectAbstractMeshId>();
      let vertex_id = vertex.query::<VertexIndex>();

      let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
      let vertex_address = vertex_addresses.index(mesh_handle).load().expand();

      let vertex_pool = binding.bind_by(&self.vertex_pool);
      unsafe {
        let position = Vec3::<f32>::sized_ty()
          .load_from_u32_buffer(
            vertex_pool,
            vertex_address.position_offset + vertex_id * val(3 * 4),
          )
          .cast_type::<Vec3<f32>>();

        let normal = Vec3::<f32>::sized_ty()
          .load_from_u32_buffer(
            vertex_pool,
            vertex_address.normal_offset + vertex_id * val(3 * 4),
          )
          .cast_type::<Vec3<f32>>();

        let uv = Vec2::<f32>::sized_ty()
          .load_from_u32_buffer(
            vertex_pool,
            vertex_address.uv_offset + vertex_id * val(2 * 4),
          )
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
}

impl DrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    todo!()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DrawCommandBuilderInvocation> {
    let node = cx.bind_by(&self.metadata);
    Box::new(BindlessDrawCreatorInDevice { node })
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
}

impl DrawCommandBuilderInvocation for BindlessDrawCreatorInDevice {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect> {
    let mesh_handle: Node<u32> = todo!();

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
