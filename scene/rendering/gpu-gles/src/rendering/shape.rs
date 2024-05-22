use rendiation_mesh_core::AttributeIndexFormat;

use crate::*;

pub trait GLESModelShapeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)>;
}

impl GLESModelShapeRenderImpl for Vec<Box<dyn GLESModelShapeRenderImpl>> {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    for provider in self {
      if let Some(com) = provider.make_component(idx) {
        return Some(com);
      }
    }
    None
  }
}

#[derive(Default)]
pub struct AttributeMeshDefaultRenderImplProvider {
  multi_access: UpdateResultToken,
  vertex: UpdateResultToken,
  index: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn GLESModelShapeRenderImpl>>
  for AttributeMeshDefaultRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx) {
    let multi_access =
      global_rev_ref().watch_inv_ref::<AttributeMeshVertexBufferRelationRefAttributeMesh>();
    self.multi_access = source.register_reactive_multi_collection(multi_access);

    let index = attribute_mesh_index_buffers(cx);
    self.index = source.register_self_contained_reactive_collection(index);

    let vertex = attribute_mesh_vertex_buffer_views(cx);
    self.vertex = source.register_self_contained_reactive_collection(vertex);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn GLESModelShapeRenderImpl> {
    Box::new(AttributeMeshDefaultRenderImpl {
      mesh_access: global_entity_component_of::<StandardModelRefAttributeMesh>().read_foreign_key(),
      mode: global_entity_component_of::<AttributeMeshTopology>().read(),
      index: res
        .take_self_contained_reactive_collection_updated(self.index)
        .unwrap(),
      vertex: AttributeMeshVertexAccessView {
        semantics: global_entity_component_of::<AttributeMeshVertexBufferSemantic>().read(),
        count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeVertexRef>>()
          .read(),
        multi_access: res
          .take_multi_reactive_collection_updated(self.multi_access)
          .unwrap(),
        vertex: res
          .take_self_contained_reactive_collection_updated(self.vertex)
          .unwrap(),
      },
      count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeIndexRef>>()
        .read(),
    })
  }
}

pub struct AttributeMeshDefaultRenderImpl {
  mesh_access: ForeignKeyReadView<StandardModelRefAttributeMesh>,
  mode: ComponentReadView<AttributeMeshTopology>,
  count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeIndexRef>>,
  index: Box<
    dyn VirtualCollectionSelfContained<EntityHandle<AttributeMeshEntity>, GPUBufferResourceView>,
  >,
  vertex: AttributeMeshVertexAccessView,
}

impl GLESModelShapeRenderImpl for AttributeMeshDefaultRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let mesh_id = self.mesh_access.get(idx)?;

    let index_buffer = self.index.access_ref(&mesh_id)?;

    let count = self.count.get_value(mesh_id)?;
    let index_info = count.map(|count| {
      let stride = u64::from(index_buffer.view_byte_size()) / count as u64;
      let fmt = match stride {
        8 => AttributeIndexFormat::Uint32,
        4 => AttributeIndexFormat::Uint16,
        _ => unreachable!("invalid index format"),
      };
      (fmt, count)
    });

    let gpu = AttributesMeshGPU {
      mode: self.mode.get_value(mesh_id)?,
      index: index_info,
      index_buffer,
      mesh_id,
      vertex: &self.vertex,
    };

    let cmd = gpu.draw_command();

    Some((Box::new(gpu), cmd))
  }
}
