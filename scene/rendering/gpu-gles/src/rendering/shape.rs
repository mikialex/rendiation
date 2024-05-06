use crate::*;

pub trait GLESModelShapeRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponentAny + '_>, DrawCommand)>;
}

impl GLESModelShapeRenderImpl for Vec<Box<dyn GLESModelShapeRenderImpl>> {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponentAny + '_>, DrawCommand)> {
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
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx) {
    let multi_access =
      global_rev_ref().watch_inv_ref_typed::<AttributeMeshVertexBufferRelationRefAttributeMesh>();
    self.multi_access = source.register_reactive_multi_collection(multi_access);

    let index = attribute_mesh_index_buffers(cx);
    self.index = source.register_self_contained_reactive_collection(index);

    let vertex = attribute_mesh_vertex_buffer_views(cx);
    self.vertex = source.register_self_contained_reactive_collection(vertex);
  }

  fn create_impl(&self, res: &ConcurrentStreamUpdateResult) -> Box<dyn GLESModelShapeRenderImpl> {
    Box::new(AttributeMeshDefaultRenderImpl {
      mesh_access: global_entity_component_of::<StandardModelRefAttributeMesh>().read(),
      mode: global_entity_component_of::<AttributeMeshTopology>().read(),
      index: res
        .get_self_contained_reactive_collection_updated(self.index)
        .unwrap(),
      vertex: AttributeMeshVertexAccessView {
        semantics: global_entity_component_of::<AttributeMeshVertexBufferSemantic>().read(),
        count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeVertexRef>>()
          .read(),
        multi_access: res
          .get_multi_reactive_collection_updated(self.multi_access)
          .unwrap(),
        vertex: res
          .get_self_contained_reactive_collection_updated(self.vertex)
          .unwrap(),
      },
    })
  }
}

pub struct AttributeMeshDefaultRenderImpl {
  mesh_access: ComponentReadView<StandardModelRefAttributeMesh>,
  mode: ComponentReadView<AttributeMeshTopology>,
  index:
    Box<dyn VirtualCollectionSelfContained<AllocIdx<AttributeMeshEntity>, GPUBufferResourceView>>,
  vertex: AttributeMeshVertexAccessView,
}

impl GLESModelShapeRenderImpl for AttributeMeshDefaultRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<(Box<dyn RenderComponentAny + '_>, DrawCommand)> {
    let idx = self.mesh_access.get(idx)?;
    let mesh_id = AllocIdx::from_alloc_index((*idx)?);

    let gpu = AttributesMeshGPU {
      mode: self.mode.get_value(mesh_id)?,
      index: todo!(),
      index_buffer: &*self.index,
      mesh_id,
      vertex: &self.vertex,
    };

    Some((Box::new(gpu), gpu.draw_command()))
  }
}
