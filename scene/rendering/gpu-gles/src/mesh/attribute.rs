use crate::*;

pub fn attribute_mesh_index_buffers(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollection<AllocIdx<AttributeMeshEntity>, GPUBufferResourceView> {
  let cx = cx.clone();
  let attribute_mesh_index_buffers = global_watch()
    .watch_typed_key::<AttributeMeshIndex>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let read_view = global_entity_component_of::<BufferEntityData>().read();
      move |_, buffer_idx| {
        let buffer = read_view.get(buffer_idx.unwrap().into()).unwrap();
        create_gpu_buffer(buffer.as_slice(), BufferUsages::INDEX, &cx.device)
      }
    });

  attribute_mesh_index_buffers
    .collective_zip(global_watch().watch_typed_key::<AttributeMeshIndexBufferRange>())
    .collective_map(|(buffer, range)| buffer.create_view(map_view(range.unwrap())))
}

pub fn attribute_mesh_vertex_buffer_views(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollection<AllocIdx<AttributeMeshVertexBufferRelation>, GPUBufferResourceView> {
  let cx = cx.clone();
  let attribute_mesh_vertex_buffers = global_watch()
    .watch_typed_key::<AttributeMeshVertexBufferRelationRefVertexBuffer>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let read_view = global_entity_component_of::<BufferEntityData>().read();
      move |_, buffer_idx| {
        let buffer = read_view.get(buffer_idx.unwrap().into()).unwrap();
        create_gpu_buffer(buffer.as_slice(), BufferUsages::VERTEX, &cx.device)
      }
    });

  attribute_mesh_vertex_buffers
    .collective_zip(global_watch().watch_typed_key::<AttributeMeshVertexBufferRange>())
    .collective_map(|(buffer, range)| buffer.create_view(map_view(range)))
}

fn map_view(view: rendiation_mesh_core::BufferViewRange) -> GPUBufferViewRange {
  GPUBufferViewRange {
    offset: view.offset,
    size: view.size,
  }
}
