use crate::*;

impl UnTypedBufferView {
  pub fn read(&self) -> UnTypedBufferViewReadView {
    UnTypedBufferViewReadView {
      buffer: &self.buffer,
      view: self,
    }
  }
}

#[derive(Clone, Copy)]
pub struct UnTypedBufferViewReadView<'a> {
  buffer: &'a Vec<u8>,
  view: &'a UnTypedBufferView,
}

impl<'a> std::ops::Deref for UnTypedBufferViewReadView<'a> {
  type Target = UnTypedBufferView;

  fn deref(&self) -> &Self::Target {
    self.view
  }
}

impl<'a> UnTypedBufferViewReadView<'a> {
  pub fn visit_bytes(&self, view_byte_offset: usize) -> Option<&[u8]> {
    let byte_slice = self.buffer.as_slice();
    let offset = self.range.offset as usize + view_byte_offset;

    if let Some(byte_size) = self.range.size {
      let byte_size = Into::<u64>::into(byte_size) as usize;
      byte_slice.get(offset..offset + byte_size)
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

#[derive(Clone, Copy)]
pub struct AttributeAccessorReadView<'a> {
  view: UnTypedBufferViewReadView<'a>,
  acc: &'a AttributeAccessor,
}

impl<'a> std::ops::Deref for AttributeAccessorReadView<'a> {
  type Target = AttributeAccessor;

  fn deref(&self) -> &Self::Target {
    self.acc
  }
}

impl<'a> AttributeAccessorReadView<'a> {
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

impl AttributeAccessor {
  pub fn read(&self) -> AttributeAccessorReadView {
    AttributeAccessorReadView {
      view: self.view.read(),
      acc: self,
    }
  }
}

#[derive(Clone)]
pub struct AttributesMeshEntityReadView<'a> {
  pub attributes:
    SmallVec<[(&'a AttributeSemantic, AttributeAccessorReadView<'a>); MOST_COMMON_ATTRIBUTE_COUNT]>,
  pub indices: Option<(AttributeIndexFormat, AttributeAccessorReadView<'a>)>,
  pub mesh: &'a AttributesMesh,
}

impl<'a> std::ops::Deref for AttributesMeshEntityReadView<'a> {
  type Target = AttributesMesh;

  fn deref(&self) -> &Self::Target {
    self.mesh
  }
}

impl<'a> AttributesMeshEntityReadView<'a> {
  pub fn primitive_count(&self) -> usize {
    let count = if let Some((_, index)) = &self.indices {
      index.count
    } else {
      self.get_position().len()
    };

    (count + self.mode.step() - self.mode.stride()) / self.mode.step()
  }

  pub fn get_attribute(&self, s: &AttributeSemantic) -> Option<&AttributeAccessorReadView> {
    self.attributes.iter().find(|(k, _)| *k == s).map(|r| &r.1)
  }
  pub fn get_position(&self) -> &[Vec3<f32>] {
    self
      .get_attribute(&AttributeSemantic::Positions)
      .expect("position attribute should always exist")
      .visit_slice::<Vec3<f32>>()
      .expect("position type is maybe not correct")
  }

  pub fn create_full_read_view_base(&self) -> FullReaderBase {
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

#[derive(Clone, Copy)]
pub struct PositionReader<'a> {
  position: &'a [Vec3<f32>],
}
impl<'a> IndexGet for PositionReader<'a> {
  type Output = Vec3<f32>;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    self.position.get(key).copied()
  }
}
pub type AttributesMeshEntityShapeReadView<'a> =
  AttributesMeshEntityCustomReadView<'a, PositionReader<'a>>;

#[derive(Clone)]
pub struct FullReaderBase<'a> {
  pub keys: Vec<AttributeSemantic>,
  pub bytes: Vec<&'a [u8]>,
}

pub type AttributesMeshEntityFullReadView<'a> =
  AttributesMeshEntityCustomReadView<'a, FullReaderBase<'a>>;

#[derive(Clone, Copy)]
pub struct FullReaderRead<'a> {
  pub reader: &'a FullReaderBase<'a>,
  pub idx: usize,
}

impl<'a> Hash for FullReaderRead<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.read_bytes().for_each(|bytes| bytes.hash(state))
  }
}

impl<'a> PartialEq for FullReaderRead<'a> {
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
impl<'a> Eq for FullReaderRead<'a> {}

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

impl<'a> IndexGet for FullReaderBase<'a> {
  type Output = FullReaderRead<'a>;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    // todo, fixme, this is not safe for now. we use the index get trait that not strong enough to
    // constraint the returned output has relation with self lifetime.
    let reader: &'a FullReaderBase<'a> = unsafe { std::mem::transmute(self) };
    // should we do option bound check here??
    Some(FullReaderRead { reader, idx: key })
  }
}

impl AttributesMesh {
  pub fn read(&self) -> AttributesMeshEntityReadView {
    let attributes = self.attributes.iter().map(|(k, a)| (k, a.read())).collect();
    let indices = self.indices.as_ref().map(|(f, a)| (*f, a.read()));

    AttributesMeshEntityReadView {
      attributes,
      indices,
      mesh: self,
    }
  }

  pub fn read_full(&self) -> AttributesMeshEntityFullReadView {
    let inner = self.read();
    let reader = inner.create_full_read_view_base();
    // safety: the returned reference is origin from the buffer itself, no cyclic reference exists
    // the allocate temp buffer is immutable and has stable heap location.
    let reader = unsafe { std::mem::transmute(reader) };
    AttributesMeshEntityFullReadView { inner, reader }
  }

  pub fn read_shape(&self) -> AttributesMeshEntityShapeReadView {
    let inner = self.read();
    let position = inner.get_position();
    // safety: the returned reference is origin from the buffer itself, no cyclic reference exists
    let position = unsafe { std::mem::transmute(position) };
    AttributesMeshEntityCustomReadView {
      inner,
      reader: PositionReader { position },
    }
  }

  pub fn get_attribute(&self, s: &AttributeSemantic) -> Option<&AttributeAccessor> {
    self.attributes.iter().find(|(k, _)| k == s).map(|r| &r.1)
  }
  pub fn get_position(&self) -> &AttributeAccessor {
    self
      .get_attribute(&AttributeSemantic::Positions)
      .expect("position attribute should always exist")
  }
}
