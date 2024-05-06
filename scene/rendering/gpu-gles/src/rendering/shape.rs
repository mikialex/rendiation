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

pub struct AttributeMeshDefaultRenderImplProvider;

impl RenderImplProvider<Box<dyn GLESModelShapeRenderImpl>>
  for AttributeMeshDefaultRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx) {
    let mesh_access = global_rev_ref().watch_inv_ref_typed::<StandardModelRefAttributeMesh>();
    todo!()
  }

  fn create_impl(&self, res: &ConcurrentStreamUpdateResult) -> Box<dyn GLESModelShapeRenderImpl> {
    Box::new(AttributeMeshDefaultRenderImpl {
      mesh_access: todo!(),
      mode: todo!(),
      index: todo!(),
      vertex: todo!(),
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
