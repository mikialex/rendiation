use dyn_downcast::*;
use rendiation_mesh_core::*;

use crate::*;

pub fn attribute_mesh_index_buffers(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollectionSelfContained<EntityHandle<AttributeMeshEntity>, GPUBufferResourceView>
{
  let cx = cx.clone();
  let attribute_mesh_index_buffers = global_watch()
    .watch::<SceneBufferViewBufferId<AttributeIndexRef>>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let read_view = global_entity_component_of::<BufferEntityData>().read();
      move |_, buffer_idx| {
        let buffer = read_view
          .get_without_generation_check(buffer_idx.unwrap().index())
          .unwrap();
        create_gpu_buffer(buffer.as_slice(), BufferUsages::INDEX, &cx.device)
      }
    });

  attribute_mesh_index_buffers
    .collective_zip(global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>())
    .collective_map(|(buffer, range)| buffer.create_view(map_view(range)))
    .materialize_unordered()
}

pub fn attribute_mesh_vertex_buffer_views(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollectionSelfContained<
  EntityHandle<AttributeMeshVertexBufferRelation>,
  GPUBufferResourceView,
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

pub struct AttributeMeshVertexAccessView {
  pub semantics: ComponentReadView<AttributeMeshVertexBufferSemantic>,
  pub count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeVertexRef>>,
  pub multi_access: Box<
    dyn DynVirtualMultiCollection<
      EntityHandle<AttributeMeshEntity>,
      EntityHandle<AttributeMeshVertexBufferRelation>,
    >,
  >,
  pub vertex: Box<
    dyn VirtualCollectionSelfContained<
      EntityHandle<AttributeMeshVertexBufferRelation>,
      GPUBufferResourceView,
    >,
  >,
}

pub struct AttributesMeshGPU<'a> {
  pub mode: rendiation_mesh_core::PrimitiveTopology,
  // fmt, count
  pub index: Option<(AttributeIndexFormat, u32)>,
  pub index_buffer: &'a GPUBufferResourceView,
  pub mesh_id: EntityHandle<AttributeMeshEntity>,
  pub vertex: &'a AttributeMeshVertexAccessView,
}

impl<'a> ShaderPassBuilder for AttributesMeshGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for vertex_info_id in self.vertex.multi_access.access_multi_value(&self.mesh_id) {
      let gpu_buffer = self.vertex.vertex.access_ref(&vertex_info_id).unwrap();
      ctx.set_vertex_buffer_owned_next(gpu_buffer);
    }
    if let Some((index_format, _)) = &self.index {
      ctx
        .pass
        .set_index_buffer_owned(self.index_buffer, map_index(*index_format))
    }
  }
}

pub trait CustomAttributeKeyGPU {
  fn inject_shader(&self, builder: &mut ShaderVertexBuilder);
}
define_dyn_trait_downcaster_static!(CustomAttributeKeyGPU);

impl<'a> ShaderHashProvider for AttributesMeshGPU<'a> {
  shader_hash_type_id! {AttributesMeshGPU<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    for vertex_info_id in self.vertex.multi_access.access_multi_value(&self.mesh_id) {
      let semantic = self.vertex.semantics.get(vertex_info_id).unwrap();
      semantic.hash(hasher)
    }
    self.mode.hash(hasher);
    if let Some((index_format, _)) = &self.index {
      if rendiation_mesh_core::PrimitiveTopology::LineStrip == self.mode
        || rendiation_mesh_core::PrimitiveTopology::TriangleStrip == self.mode
      {
        index_format.hash(hasher)
      }
    }
  }
}
impl<'a> GraphicsShaderProvider for AttributesMeshGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
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
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Joints(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<JointIndexChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<JointIndexChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<JointIndexChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<JointIndexChannel<3>>(mode),
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Weights(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<WeightChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<WeightChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<WeightChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<WeightChannel<3>>(mode),
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Foreign(key) => {
            get_dyn_trait_downcaster_static!(CustomAttributeKeyGPU)
              .downcast_ref(key.implementation.as_ref())
              .ok_or(ShaderBuildError::SemanticNotSupported)?
              .inject_shader(builder)
          }
        }
      }
      builder.primitive_state.topology = map_topology(self.mode);
      Ok(())
    })
  }
}

impl<'a> AttributesMeshGPU<'a> {
  pub fn draw_command(&self) -> DrawCommand {
    if let Some((_, count)) = &self.index {
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

pub fn map_topology(
  pt: rendiation_mesh_core::PrimitiveTopology,
) -> rendiation_webgpu::PrimitiveTopology {
  use rendiation_mesh_core::PrimitiveTopology as Enum;
  use rendiation_webgpu::PrimitiveTopology as GPUEnum;
  match pt {
    Enum::PointList => GPUEnum::PointList,
    Enum::LineList => GPUEnum::LineList,
    Enum::LineStrip => GPUEnum::LineStrip,
    Enum::TriangleList => GPUEnum::TriangleList,
    Enum::TriangleStrip => GPUEnum::TriangleStrip,
  }
}

pub fn map_index(index: AttributeIndexFormat) -> IndexFormat {
  match index {
    AttributeIndexFormat::Uint16 => IndexFormat::Uint16,
    AttributeIndexFormat::Uint32 => IndexFormat::Uint32,
  }
}
