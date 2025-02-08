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
pub struct AttributesMeshEntityDefaultRenderImplProvider {
  multi_access: UpdateResultToken,
  vertex: UpdateResultToken,
  index: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn GLESModelShapeRenderImpl>>
  for AttributesMeshEntityDefaultRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let multi_access = global_rev_ref()
      .watch_inv_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(
    );
    self.multi_access = source.register_multi_reactive_query(multi_access);

    let index = attribute_mesh_index_buffers(cx);
    self.index = source.register_val_refed_reactive_query(index);

    let vertex = attribute_mesh_vertex_buffer_views(cx);
    self.vertex = source.register_val_refed_reactive_query(vertex);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.multi_access);
    source.deregister(&mut self.index);
    source.deregister(&mut self.vertex);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn GLESModelShapeRenderImpl> {
    Box::new(AttributesMeshEntityDefaultRenderImpl {
      mesh_access: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      mode: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
      index: res
        .take_val_refed_reactive_query_updated(self.index)
        .unwrap(),
      vertex: AttributesMeshEntityVertexAccessView {
        semantics: global_entity_component_of::<AttributesMeshEntityVertexBufferSemantic>().read(),
        count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeVertexRef>>()
          .read(),
        multi_access: res
          .take_reactive_multi_query_updated(self.multi_access)
          .unwrap(),
        vertex: res
          .take_val_refed_reactive_query_updated(self.vertex)
          .unwrap(),
      },
      count: global_entity_component_of::<SceneBufferViewBufferItemCount<AttributeIndexRef>>()
        .read(),
    })
  }
}

pub struct AttributesMeshEntityDefaultRenderImpl {
  mesh_access: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  mode: ComponentReadView<AttributesMeshEntityTopology>,
  count: ComponentReadView<SceneBufferViewBufferItemCount<AttributeIndexRef>>,
  index: BoxedDynValueRefQuery<EntityHandle<AttributesMeshEntity>, GPUBufferResourceView>,
  vertex: AttributesMeshEntityVertexAccessView,
}

impl GLESModelShapeRenderImpl for AttributesMeshEntityDefaultRenderImpl {
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
    };

    let cmd = gpu.draw_command();

    Some((Box::new(gpu), cmd))
  }
}
