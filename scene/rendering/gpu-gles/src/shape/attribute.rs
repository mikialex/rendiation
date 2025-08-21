use rendiation_mesh_core::*;

use crate::*;

pub fn use_attribute_mesh_renderer(
  cx: &mut QueryGPUHookCx,
  foreign_implementation_semantics: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
) -> Option<GLESAttributesMeshRenderer> {
  let multi_access =
    cx.use_db_rev_ref_typed::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>();

  let index = use_buffers::<AttributeIndexRef>(cx, BufferUsages::INDEX);
  let vertex = use_buffers::<AttributeVertexRef>(cx, BufferUsages::VERTEX);

  cx.when_render(|| GLESAttributesMeshRenderer {
    mesh_access: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
      .read_foreign_key(),
    mode: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
    index: index.make_read_holder(),
    vertex: AttributesMeshEntityVertexAccessView {
      semantics: global_entity_component_of::<AttributesMeshEntityVertexBufferSemantic>().read(),
      count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeVertexRef>>()
        .read(),
      multi_access: multi_access.expect_resolve_stage(),
      vertex: vertex.make_read_holder(),
    },
    count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeIndexRef>>().read(),
    foreign_implementation_semantics,
  })
}

pub struct GLESAttributesMeshRenderer {
  mesh_access: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  mode: ComponentReadView<AttributesMeshEntityTopology>,
  count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeIndexRef>>,
  index: BufferCollectionRead,
  vertex: AttributesMeshEntityVertexAccessView,
  foreign_implementation_semantics: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
}

impl GLESModelShapeRenderImpl for GLESAttributesMeshRenderer {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let mesh_id = self.mesh_access.get(idx)?;

    let index = if let Some(index_buffer) = self.index.access_ref(&mesh_id.into_raw()) {
      let count = self.count.get_value(mesh_id).unwrap().unwrap() as u64;
      let stride = u64::from(index_buffer.view_byte_size()) / count;
      let fmt = match stride {
        4 => AttributeIndexFormat::Uint32,
        2 => AttributeIndexFormat::Uint16,
        _ => unreachable!("invalid index format, computed stride size {}", stride),
      };
      (fmt, count as u32, index_buffer).into()
    } else {
      None
    };

    let gpu = AttributesMeshGPU {
      mode: self.mode.get_value(mesh_id)?,
      index,
      mesh_id,
      vertex: &self.vertex,
      foreign_implementation_semantics: self.foreign_implementation_semantics.clone(),
    };

    let cmd = gpu.draw_command();

    Some((Box::new(gpu), cmd))
  }
}

type BufferCollection = SharedHashMap<RawEntityHandle, GPUBufferResourceView>;
type BufferCollectionRead = SharedHashMapRead<RawEntityHandle, GPUBufferResourceView>;

fn use_buffers<B: SceneBufferView>(
  cx: &mut QueryGPUHookCx,
  usage: BufferUsages,
) -> BufferCollection {
  let map = cx.use_shared_hash_map();

  let source = cx
    .use_dual_query::<SceneBufferViewBufferId<B>>()
    .dual_query_zip(cx.use_dual_query::<SceneBufferViewBufferRange<B>>())
    .into_delta_change()
    .filter_map_changes(|(id, range)| Some((id?, range)));

  let read_view = global_entity_component_of::<BufferEntityData>().read();

  let f = |(idx, range): (RawEntityHandle, Option<BufferViewRange>)| {
    let buffer = unsafe { read_view.get_by_untyped_handle(idx).unwrap() };
    let buffer = create_gpu_buffer(buffer.as_slice(), usage, &cx.gpu.device);
    buffer.create_view(map_view(range))
  };

  maintain_shared_map(&map, source, f);

  map
}

fn map_view(view: Option<rendiation_mesh_core::BufferViewRange>) -> GPUBufferViewRange {
  view
    .map(|view| GPUBufferViewRange {
      offset: view.offset,
      size: view.size,
    })
    .unwrap_or_default()
}

pub struct AttributesMeshEntityVertexAccessView {
  pub semantics: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,
  pub count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeVertexRef>>,
  pub multi_access:
    RevRefForeignKeyReadTyped<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub vertex: BufferCollectionRead,
}

pub struct AttributesMeshGPU<'a> {
  pub mode: rendiation_mesh_core::PrimitiveTopology,
  // fmt, count, buffer
  pub index: Option<(AttributeIndexFormat, u32, &'a GPUBufferResourceView)>,
  pub mesh_id: EntityHandle<AttributesMeshEntity>,
  pub vertex: &'a AttributesMeshEntityVertexAccessView,
  pub foreign_implementation_semantics: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
}

impl ShaderPassBuilder for AttributesMeshGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for vertex_info_id in self.vertex.multi_access.access_multi_value(&self.mesh_id) {
      let gpu_buffer = self
        .vertex
        .vertex
        .access_ref(&vertex_info_id.into_raw())
        .unwrap();
      ctx.set_vertex_buffer_by_buffer_resource_view_next(gpu_buffer);
    }
    if let Some((index_format, _, buffer)) = &self.index {
      ctx
        .pass
        .set_index_buffer_by_buffer_resource_view(buffer, map_index(*index_format))
    }
  }
}

impl ShaderHashProvider for AttributesMeshGPU<'_> {
  shader_hash_type_id! {AttributesMeshGPU<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    for vertex_info_id in self.vertex.multi_access.access_multi_value(&self.mesh_id) {
      let semantic = self.vertex.semantics.get(vertex_info_id).unwrap();
      semantic.hash(hasher)
    }
    self.mode.hash(hasher);
  }
}
impl GraphicsShaderProvider for AttributesMeshGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let mode = VertexStepMode::Vertex;
    builder.vertex(|builder, _| {
      for vertex_info_id in self.vertex.multi_access.access_multi_value(&self.mesh_id) {
        let s = self.vertex.semantics.get(vertex_info_id).unwrap();

        match s {
          AttributeSemantic::Positions => {
            builder.push_single_vertex_layout::<GeometryPosition>(mode)
          }
          AttributeSemantic::Normals => builder.push_single_vertex_layout::<GeometryNormal>(mode),
          AttributeSemantic::Tangents => builder.push_single_vertex_layout::<GeometryTangent>(mode),
          AttributeSemantic::Colors(_) => builder.push_single_vertex_layout::<GeometryColor>(mode),
          AttributeSemantic::TexCoords(channel) => match channel {
            // support 3 channel should be enough
            0 => builder.push_single_vertex_layout::<GeometryUVChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<GeometryUVChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<GeometryUVChannel<2>>(mode),
            _ => builder.error(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Joints(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<JointIndexChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<JointIndexChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<JointIndexChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<JointIndexChannel<3>>(mode),
            _ => builder.error(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Weights(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<WeightChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<WeightChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<WeightChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<WeightChannel<3>>(mode),
            _ => builder.error(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Foreign {
            implementation_id, ..
          } => {
            (self.foreign_implementation_semantics)(*implementation_id, builder);
          }
        }
      }
      builder.primitive_state.topology = map_topology(self.mode);
    })
  }
}

impl AttributesMeshGPU<'_> {
  pub fn draw_command(&self) -> DrawCommand {
    if let Some((_, count, _)) = &self.index {
      DrawCommand::Indexed {
        base_vertex: 0,
        indices: 0..*count,
        instances: 0..1,
      }
    } else {
      let first_vertex = self
        .vertex
        .multi_access
        .access_multi_value(&self.mesh_id)
        .next()
        .unwrap();

      let count = *self.vertex.count.get(first_vertex).unwrap();

      DrawCommand::Array {
        vertices: 0..count.unwrap(),
        instances: 0..1,
      }
    }
  }
}
