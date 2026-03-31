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
