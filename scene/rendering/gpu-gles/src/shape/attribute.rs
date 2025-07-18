use rendiation_mesh_core::*;

use crate::*;

pub fn use_attribute_mesh_renderer(
  cx: &mut impl QueryGPUHookCx,
  foreign_implementation_semantics: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
) -> Option<GLESAttributesMeshRenderer> {
  let multi_access = cx.use_global_multi_reactive_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>();

  let index = cx.use_val_refed_reactive_query(attribute_mesh_index_buffers);
  let vertex = cx.use_val_refed_reactive_query(attribute_mesh_vertex_buffer_views);

  cx.when_render(|| GLESAttributesMeshRenderer {
    mesh_access: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
      .read_foreign_key(),
    mode: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
    index: index.unwrap(),
    vertex: AttributesMeshEntityVertexAccessView {
      semantics: global_entity_component_of::<AttributesMeshEntityVertexBufferSemantic>().read(),
      count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeVertexRef>>()
        .read(),
      multi_access: multi_access.unwrap(),
      vertex: vertex.unwrap(),
    },
    count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeIndexRef>>().read(),
    foreign_implementation_semantics,
  })
}

pub struct GLESAttributesMeshRenderer {
  mesh_access: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  mode: ComponentReadView<AttributesMeshEntityTopology>,
  count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeIndexRef>>,
  index: BoxedDynValueRefQuery<EntityHandle<AttributesMeshEntity>, GPUBufferResourceView>,
  vertex: AttributesMeshEntityVertexAccessView,
  foreign_implementation_semantics: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
}

impl GLESModelShapeRenderImpl for GLESAttributesMeshRenderer {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let mesh_id = self.mesh_access.get(idx)?;

    let index = if let Some(index_buffer) = self.index.access_ref(&mesh_id) {
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

fn attribute_mesh_index_buffers(
  cx: &GPU,
) -> impl ReactiveValueRefQuery<Key = EntityHandle<AttributesMeshEntity>, Value = GPUBufferResourceView>
{
  let cx = cx.clone();
  let attribute_mesh_index_buffers = global_watch()
    .watch::<SceneBufferViewBufferId<AttributeIndexRef>>()
    .collective_filter_map(|b| b)
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let read_view = global_entity_component_of::<BufferEntityData>().read();
      move |_, buffer_idx| {
        let buffer = read_view
          .get_without_generation_check(buffer_idx.index())
          .unwrap();
        create_gpu_buffer(buffer.as_slice(), BufferUsages::INDEX, &cx.device)
      }
    });

  attribute_mesh_index_buffers
    .collective_union(
      global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>(),
      |(buffer, range)| buffer.map(|buffer| buffer.create_view(map_view(range.flatten()))),
    )
    .materialize_unordered()
}

fn attribute_mesh_vertex_buffer_views(
  cx: &GPU,
) -> impl ReactiveValueRefQuery<
  Key = EntityHandle<AttributesMeshEntityVertexBufferRelation>,
  Value = GPUBufferResourceView,
> {
  let cx = cx.clone();
  let attribute_mesh_vertex_buffers = global_watch()
    .watch::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let read_view = global_entity_component_of::<BufferEntityData>().read();
      move |_, buffer_idx| {
        let buffer = read_view
          .get_without_generation_check(buffer_idx.unwrap().index())
          .unwrap();
        create_gpu_buffer(buffer.as_slice(), BufferUsages::VERTEX, &cx.device)
      }
    });

  attribute_mesh_vertex_buffers
    .collective_zip(global_watch().watch::<SceneBufferViewBufferRange<AttributeVertexRef>>())
    .collective_map(|(buffer, range)| buffer.create_view(map_view(range)))
    .materialize_unordered()
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
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub vertex: BoxedDynValueRefQuery<
    EntityHandle<AttributesMeshEntityVertexBufferRelation>,
    GPUBufferResourceView,
  >,
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
      let gpu_buffer = self.vertex.vertex.access_ref(&vertex_info_id).unwrap();
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
