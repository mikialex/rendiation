use crate::*;

#[no_mangle]
pub extern "C" fn create_mesh(
  indices_length: u32,
  indices: *const u32,
  vertex_length: u32,
  position: *const f32,
  normal_raw: *const f32,
  uv_raw: *const f32,
  topo: MeshPrimitiveTopology,
) -> AttributesMeshEntitiesCommon {
  let indices = unsafe { slice::from_raw_parts(indices, indices_length as usize) };
  let indices: &[u8] = bytemuck::cast_slice(indices);
  let indices = indices.to_vec();

  let mut attributes = Vec::new();

  let position = unsafe { slice::from_raw_parts(position, vertex_length as usize * 3) };
  let position: &[u8] = bytemuck::cast_slice(position);
  let position = position.to_vec();
  attributes.push((AttributeSemantic::Positions, position));

  let has_normal = !normal_raw.is_null();
  if has_normal {
    let normal = unsafe { slice::from_raw_parts(normal_raw, vertex_length as usize * 3) };
    let normal: &[u8] = bytemuck::cast_slice(normal);
    let normal = normal.to_vec();
    attributes.push((AttributeSemantic::Normals, normal));
  }

  let has_uv = !uv_raw.is_null();
  if has_uv {
    let uv = unsafe { slice::from_raw_parts(uv_raw, vertex_length as usize * 2) };
    let uv: &[u8] = bytemuck::cast_slice(uv);
    let uv = uv.to_vec();
    attributes.push((AttributeSemantic::TexCoords(0), uv));
  }

  let mut writer = AttributesMeshEntityFromAttributesMeshWriter::from_global();
  let mut buffer = global_entity_of::<BufferEntity>().entity_writer();
  let mesh = AttributesMeshData {
    attributes,
    indices: Some((AttributeIndexFormat::Uint32, indices)),
    mode: topo,
  }
  .build()
  .write(&mut writer, &mut buffer);

  // it's not good
  let (normal, uv) = match (has_normal, has_uv) {
    (true, true) => (
      VertexPair::from_typed(mesh.vertices[1]),
      VertexPair::from_typed(mesh.vertices[2]),
    ),
    (true, false) => (
      VertexPair::from_typed(mesh.vertices[1]),
      VertexPair::empty(),
    ),
    (false, true) => (
      VertexPair::empty(),
      VertexPair::from_typed(mesh.vertices[1]),
    ),
    (false, false) => (VertexPair::empty(), VertexPair::empty()),
  };

  AttributesMeshEntitiesCommon {
    mesh: mesh.mesh.into(),
    index: mesh.index.unwrap().into(),
    position: VertexPair::from_typed(mesh.vertices[0]),
    normal,
    uv,
    has_normal,
    has_uv,
  }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VertexPair {
  h1: ViewerEntityHandle,
  h2: ViewerEntityHandle,
}

impl VertexPair {
  fn empty() -> Self {
    Self {
      h1: ViewerEntityHandle::empty(),
      h2: ViewerEntityHandle::empty(),
    }
  }
  fn from_typed(
    handle: (
      EntityHandle<AttributesMeshEntityVertexBufferRelation>,
      EntityHandle<BufferEntity>,
    ),
  ) -> Self {
    VertexPair {
      h1: handle.0.into(),
      h2: handle.1.into(),
    }
  }
  fn into_typed(
    self,
  ) -> (
    EntityHandle<AttributesMeshEntityVertexBufferRelation>,
    EntityHandle<BufferEntity>,
  ) {
    (self.h1.into(), self.h2.into())
  }
}

#[repr(C)]
pub struct AttributesMeshEntitiesCommon {
  mesh: ViewerEntityHandle,
  index: ViewerEntityHandle,
  position: VertexPair,
  normal: VertexPair,
  has_normal: bool,
  uv: VertexPair,
  has_uv: bool,
}

#[no_mangle]
pub extern "C" fn drop_mesh(entities: AttributesMeshEntitiesCommon) {
  let mut writer = AttributesMeshEntityFromAttributesMeshWriter::from_global();
  let mut buffer = global_entity_of::<BufferEntity>().entity_writer();

  let mut vertices = Vec::new();

  vertices.push(entities.position.into_typed());
  if entities.has_normal {
    vertices.push(entities.normal.into_typed());
  }
  if entities.has_uv {
    vertices.push(entities.uv.into_typed());
  }

  let entities: AttributesMeshEntities = AttributesMeshEntities {
    mesh: entities.mesh.into(),
    index: Some(entities.index.into()),
    vertices: vertices.into(),
  };
  entities.clean_up(&mut writer, &mut buffer);
}

fn create_vertex_attribute(
  byte_size: u32,
  item_byte_size: u32,
  semantic: AttributeSemantic,
  data: &ExternalRefPtr<MaybeUriData<Arc<Vec<u8>>>>,
  mesh_handle: ViewerEntityHandle,
) -> VertexPair {
  let mut buffer_writer = global_entity_of::<BufferEntity>().entity_writer();
  let buffer_handle = buffer_writer.new_entity(|w| w.write::<BufferEntityData>(data));
  let mut mesh_writer = AttributesMeshEntityFromAttributesMeshWriter::from_global();
  let mesh_handle: EntityHandle<AttributesMeshEntity> = mesh_handle.into();
  let vertex_view = SceneBufferViewDataView {
    data: Some(buffer_handle),
    range: None,
    count: byte_size / item_byte_size,
  };
  let relation = mesh_writer.relation.new_entity(|w| {
    let w = w
      .write::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(
        &mesh_handle.some_handle(),
      )
      .write::<AttributesMeshEntityVertexBufferSemantic>(&semantic);
    vertex_view.write::<AttributeVertexRef>(w)
  });
  VertexPair::from_typed((relation, buffer_handle))
}

#[repr(C)]
pub enum MeshAPIDataType {
  Position,
  Normal,
  Uv,
  Indices,
}

#[no_mangle]
pub extern "C" fn update_mesh_data(
  entities: &mut AttributesMeshEntitiesCommon,
  byte_size: u32,
  data: *const f32,
  vertex_ty: MeshAPIDataType,
) {
  if byte_size == 0 || data.is_null() {
    log::warn!("update_mesh_data: byte_size is 0 or data is null");
    return;
  }

  let element_byte_size = match vertex_ty {
    MeshAPIDataType::Position | MeshAPIDataType::Normal => 12,
    MeshAPIDataType::Uv => 8,
    MeshAPIDataType::Indices => 4,
  };
  if byte_size % element_byte_size != 0 {
    log::warn!(
      "update_mesh_data: byte_size {byte_size} is not divisible by element size {element_byte_size}"
    );
    return;
  }

  let data = unsafe { slice::from_raw_parts(data as *const u8, byte_size as usize) };
  let data = ExternalRefPtr::new(MaybeUriData::Living(Arc::new(data.to_vec())));

  let mut buffer_writer = global_entity_of::<BufferEntity>().entity_writer();

  match vertex_ty {
    MeshAPIDataType::Position => {
      let buffer_handle: EntityHandle<BufferEntity> = entities.position.h2.into();
      buffer_writer.write::<BufferEntityData>(buffer_handle, data);
    }
    MeshAPIDataType::Normal => {
      if entities.has_normal {
        let buffer_handle: EntityHandle<BufferEntity> = entities.normal.h2.into();
        buffer_writer.write::<BufferEntityData>(buffer_handle, data);
      } else {
        entities.normal = create_vertex_attribute(
          byte_size,
          12,
          AttributeSemantic::Normals,
          &data,
          entities.mesh,
        );
        entities.has_normal = true;
      }
    }
    MeshAPIDataType::Uv => {
      if entities.has_uv {
        let buffer_handle: EntityHandle<BufferEntity> = entities.uv.h2.into();
        buffer_writer.write::<BufferEntityData>(buffer_handle, data);
      } else {
        entities.uv = create_vertex_attribute(
          byte_size,
          8,
          AttributeSemantic::TexCoords(0),
          &data,
          entities.mesh,
        );
        entities.has_uv = true;
      }
    }
    MeshAPIDataType::Indices => {
      let buffer_handle: EntityHandle<BufferEntity> = entities.index.into();
      buffer_writer.write::<BufferEntityData>(buffer_handle, data);
    }
  }
}

#[no_mangle]
pub extern "C" fn set_mesh_topology(mesh: ViewerEntityHandle, topo: MeshPrimitiveTopology) {
  write_global_db_component::<AttributesMeshEntityTopology>().write(mesh.into(), topo);
}
