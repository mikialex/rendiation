use crate::*;

impl UnTypedBufferView {
  pub fn visit_bytes(&self, view_byte_offset: usize) -> Option<&[u8]> {
    let byte_slice = self.buffer.as_slice();
    let offset = self.range.offset as usize + view_byte_offset;

    if let Some(byte_size) = self.range.size {
      let end = self.range.offset + byte_size.get();
      byte_slice.get(offset..end as usize)
    } else {
      byte_slice.get(offset..)
    }
  }

  pub fn visit_slice<T: bytemuck::Pod>(
    &self,
    view_byte_offset: usize,
    typed_count: usize,
  ) -> Option<&[T]> {
    let byte_slice = self.visit_bytes(view_byte_offset)?;
    let cast_slice = bytemuck::try_cast_slice(byte_slice).ok()?;
    cast_slice.get(0..typed_count)
  }

  pub fn get<T: bytemuck::Pod>(
    &self,
    view_byte_offset: usize,
    typed_count: usize,
    index: usize,
  ) -> Option<T> {
    self
      .visit_slice(view_byte_offset, typed_count)?
      .get(index)
      .cloned()
  }
}

impl AttributeAccessor {
  pub fn visit_bytes(&self) -> Option<&[u8]> {
    self.view.visit_bytes(self.byte_offset)
  }

  pub fn visit_slice<T: bytemuck::Pod>(&self) -> Option<&[T]> {
    self.view.visit_slice(self.byte_offset, self.count)
  }
  pub fn get<T: bytemuck::Pod>(&self, index: usize) -> Option<T> {
    self.view.get(self.byte_offset, self.count, index)
  }
}

#[derive(Clone)]
pub struct FullReaderBase<'a> {
  pub keys: Vec<AttributeSemantic>,
  pub bytes: Vec<&'a [u8]>,
}

impl<'a> IndexGet for FullReaderBase<'a> {
  type Output = FullReaderRead<'a>;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    // todo, we use the index get trait that not strong enough to
    // constraint the returned output has relation with self lifetime.
    let reader: &'a FullReaderBase<'a> = unsafe { std::mem::transmute(self) };
    Some(FullReaderRead { reader, idx: key })
  }
}

#[derive(Clone, Copy)]
pub struct FullReaderRead<'a> {
  pub reader: &'a FullReaderBase<'a>,
  pub idx: usize,
}

impl AttributeVertex for FullReaderRead<'_> {
  fn write(self, target: &mut [Vec<u8>]) {
    for (source, target) in self.read_bytes().zip(target.iter_mut()) {
      target.extend_from_slice(source)
    }
  }

  fn create_layout(&self) -> Vec<AttributeSemantic> {
    self.reader.keys.clone()
  }
}

impl Hash for FullReaderRead<'_> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.read_bytes().for_each(|bytes| bytes.hash(state))
  }
}

impl PartialEq for FullReaderRead<'_> {
  fn eq(&self, other: &Self) -> bool {
    if self.reader.keys.len() != other.reader.keys.len() {
      return false;
    }
    for (a, b) in self.read_bytes().zip(other.read_bytes()) {
      if a != b {
        return false;
      }
    }
    true
  }
}
impl Eq for FullReaderRead<'_> {}

impl<'a> FullReaderRead<'a> {
  pub fn read_bytes(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
    self
      .reader
      .keys
      .iter()
      .zip(&self.reader.bytes)
      .map(|(k, source)| {
        let byte_size = k.item_byte_size();
        source
          .get(self.idx * byte_size..(self.idx + 1) * byte_size)
          .unwrap()
      })
  }
}

impl AttributesMesh {
  pub fn get_attribute(&self, s: &AttributeSemantic) -> Option<&AttributeAccessor> {
    self.attributes.iter().find(|(k, _)| k == s).map(|r| &r.1)
  }

  pub fn get_position(&self) -> &AttributeAccessor {
    self
      .get_attribute(&AttributeSemantic::Positions)
      .expect("position attribute should always exist")
  }

  pub fn get_position_slice(&self) -> &[Vec3<f32>] {
    self
      .get_position()
      .visit_slice::<Vec3<f32>>()
      .expect("unexpected position type")
  }

  pub fn primitive_count(&self) -> usize {
    let count = if let Some((_, index)) = &self.indices {
      index.count
    } else {
      self.get_position_slice().len()
    };

    (count + self.mode.step() - self.mode.stride()) / self.mode.step()
  }

  pub fn create_abstract_mesh_view<V>(
    &self,
    vertices_reader: V,
  ) -> AttributesMeshEntityAbstractMeshReadView<V, DynIndexView<'_>> {
    AttributesMeshEntityAbstractMeshReadView {
      mode: self.mode,
      vertices: vertices_reader,
      indices: self.indices.as_ref().map(|index| DynIndexView {
        fmt: index.0,
        buffer: &index.1,
      }),
      count: self.primitive_count(),
    }
  }

  pub fn create_full_read_view_base(&self) -> FullReaderBase<'_> {
    FullReaderBase {
      keys: self.attributes.iter().map(|(k, _)| (*k).clone()).collect(),
      bytes: self
        .attributes
        .iter()
        .map(|(_, b)| b.visit_bytes().unwrap())
        .collect(),
    }
  }
}

pub struct DynIndexView<'a> {
  fmt: AttributeIndexFormat,
  buffer: &'a AttributeAccessor,
}

impl IndexGet for DynIndexView<'_> {
  type Output = usize;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    match self.fmt {
      AttributeIndexFormat::Uint16 => self.buffer.visit_slice::<u16>()?.index_get(key)? as usize,
      AttributeIndexFormat::Uint32 => self.buffer.visit_slice::<u32>()?.index_get(key)? as usize,
    }
    .into()
  }
}

pub struct AttributesMeshEntityAbstractMeshReadView<T, I> {
  pub mode: PrimitiveTopology,
  pub vertices: T,
  pub indices: Option<I>,
  pub count: usize,
}

/// we can not impl AbstractMesh for AttributesMeshEntity because it contains interior mutability.
///
/// this is slow, but not bloat the binary size.
impl<V, T, I> AbstractMesh for AttributesMeshEntityAbstractMeshReadView<T, I>
where
  T: IndexGet<Output = V>,
  I: IndexGet<Output = usize>,
  V: Copy,
{
  type Primitive = AttributeDynPrimitive<V>;

  fn primitive_count(&self) -> usize {
    self.count
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let read_index = self.mode.step() * primitive_index;

    #[rustfmt::skip]
     if let Some(index) = &self.indices {
      match self.mode {
        PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(index, read_index)?.f_filter_map(|id|self.vertices.index_get(id))?),
        PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(index, read_index)?.f_filter_map(|id|self.vertices.index_get(id))?),
        PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(index, read_index)?.f_filter_map(|id|self.vertices.index_get(id))?),
        PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(index, read_index)?.f_filter_map(|id|self.vertices.index_get(id))?),
        PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(index, read_index)?.f_filter_map(|id|self.vertices.index_get(id))?),
      }.into()
    } else {
      match self.mode {
        PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&self.vertices, read_index)?),
        PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&self.vertices, read_index)?),
        PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&self.vertices, read_index)?),
        PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&self.vertices, read_index)?),
        PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&self.vertices, read_index)?),
      }.into()
    }
  }
}

pub enum AttributeDynPrimitive<T = Vec3<f32>> {
  Points(Point<T>),
  LineSegment(LineSegment<T>),
  Triangle(Triangle<T>),
}

impl SpaceEntity<f32, 3> for AttributeDynPrimitive {
  type Matrix = Mat4<f32>;

  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    match self {
      AttributeDynPrimitive::Points(v) => {
        v.apply_matrix(mat);
      }
      AttributeDynPrimitive::LineSegment(v) => {
        v.apply_matrix(mat);
      }
      AttributeDynPrimitive::Triangle(v) => {
        v.apply_matrix(mat);
      }
    }
    self
  }
}
