use std::{
  any::{Any, TypeId},
  hash::Hash,
  num::NonZeroU64,
  sync::Arc,
};

use dyn_downcast::*;
use rendiation_algebra::Vec3;
use smallvec::SmallVec;

use crate::{IndexGet, MeshGroupsInfo, PrimitiveTopology};

mod merge;
mod picking;

pub use merge::*;
pub use picking::*;

/// Vertex attribute semantic name.
///
/// https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Default)]
pub enum AttributeSemantic {
  /// XYZ vertex positions.
  #[default]
  Positions,

  /// XYZ vertex normals.
  Normals,

  /// XYZW vertex tangents where the `w` component is a sign value indicating the
  /// handedness of the tangent basis.
  Tangents,

  /// RGB or RGBA vertex color.
  Colors(u32),

  /// UV texture co-ordinates.
  TexCoords(u32),

  /// Joint indices.
  Joints(u32),

  /// Joint weights.
  Weights(u32),

  Foreign(ForeignAttributeKey),
}

pub trait AttributeReadSchema {
  fn item_byte_size(&self) -> usize;
}
define_dyn_trait_downcaster_static!(AttributeReadSchema);

impl AttributeReadSchema for AttributeSemantic {
  fn item_byte_size(&self) -> usize {
    match self {
      AttributeSemantic::Positions => 3 * 4,
      AttributeSemantic::Normals => 3 * 4,
      AttributeSemantic::Tangents => 4 * 4,
      AttributeSemantic::Colors(_) => 4 * 4,
      AttributeSemantic::TexCoords(_) => 2 * 4,
      AttributeSemantic::Joints(_) => 4 * 2,
      AttributeSemantic::Weights(_) => 4 * 4,
      AttributeSemantic::Foreign(key) => get_dyn_trait_downcaster_static!(AttributeReadSchema)
        .downcast_ref(key.implementation.as_ref())
        .unwrap() // this is safe to unwrap, because it's bounded in ForeignAttributeKey new method
        .item_byte_size(),
    }
  }
}

#[derive(Clone)]
pub struct ForeignAttributeKey {
  id: TypeId,
  pub implementation: Arc<dyn Any + Send + Sync>,
}

impl ForeignAttributeKey {
  pub fn new<T>(implementation: T) -> Self
  where
    T: std::any::Any
      + Clone
      + Send
      + Sync
      + AsRef<dyn AttributeReadSchema>
      + AsMut<dyn AttributeReadSchema>,
  {
    get_dyn_trait_downcaster_static!(AttributeReadSchema).register::<T>();
    Self {
      id: implementation.type_id(),
      implementation: Arc::new(implementation),
    }
  }
}

impl std::fmt::Debug for ForeignAttributeKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ForeignAttributeKey")
      .field("id", &self.id)
      .finish()
  }
}

impl Eq for ForeignAttributeKey {}
impl PartialEq for ForeignAttributeKey {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Ord for ForeignAttributeKey {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.id.cmp(&other.id)
  }
}
impl PartialOrd for ForeignAttributeKey {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl std::hash::Hash for ForeignAttributeKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

#[derive(Clone)]
pub struct GeometryBufferImpl {
  pub buffer: Vec<u8>,
}

pub type GeometryBuffer = Arc<GeometryBufferImpl>;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct BufferViewRange {
  /// in bytes
  pub offset: u64,
  /// in bytes, Size of the binding, or None for using the rest of the buffer.
  pub size: Option<std::num::NonZeroU64>,
}

/// like slice, but owned, ref counted cheap clone
#[derive(Clone)]
pub struct UnTypedBufferView {
  pub buffer: GeometryBuffer,
  pub range: BufferViewRange,
  // pub byte_stride: Option<usize>, todo
}

impl UnTypedBufferView {
  pub fn read(&self) -> UnTypedBufferViewReadView {
    UnTypedBufferViewReadView {
      buffer: &self.buffer,
      view: self,
    }
  }
}

pub struct UnTypedBufferViewReadView<'a> {
  buffer: &'a GeometryBufferImpl,
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
    let byte_slice = self.buffer.buffer.as_slice();
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

#[derive(Clone)]
pub struct AttributeAccessor {
  pub view: UnTypedBufferView,
  /// offset relative to the view
  pub byte_offset: usize,
  pub count: usize,
  /// corespondent to the data type
  /// for example: vec3<f32> => 3 * 4
  pub item_byte_size: usize,
}

impl AttributeAccessor {
  pub fn create_owned<T: bytemuck::Pod>(input: Vec<T>, item_byte_size: usize) -> Self {
    let buffer = bytemuck::cast_slice(&input).to_owned();
    let count = buffer.len() / item_byte_size;

    let buffer = GeometryBufferImpl { buffer };
    let buffer = Arc::new(buffer);
    let view = UnTypedBufferView {
      buffer,
      range: Default::default(),
    };
    Self {
      view,
      byte_offset: 0,
      count,
      item_byte_size,
    }
  }
}

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

impl AttributeAccessor {
  pub fn compute_gpu_buffer_range(&self) -> BufferViewRange {
    let inner_offset = self.view.range.offset;
    BufferViewRange {
      offset: inner_offset + self.byte_offset as u64,
      size: NonZeroU64::new((self.count * self.item_byte_size) as u64)
        .unwrap() // safe
        .into(),
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum AttributeIndexFormat {
  /// Indices are 16 bit unsigned integers.
  Uint16 = 0,
  /// Indices are 32 bit unsigned integers.
  Uint32 = 1,
}

impl AttributeIndexFormat {
  pub fn byte_size(&self) -> usize {
    match self {
      AttributeIndexFormat::Uint16 => 2,
      AttributeIndexFormat::Uint32 => 4,
    }
  }
}

pub const MOST_COMMON_ATTRIBUTE_COUNT: usize = 3;

#[derive(Clone)]
pub struct AttributesMesh {
  pub attributes: SmallVec<[(AttributeSemantic, AttributeAccessor); MOST_COMMON_ATTRIBUTE_COUNT]>,
  pub indices: Option<(AttributeIndexFormat, AttributeAccessor)>,
  pub mode: PrimitiveTopology,
  pub groups: MeshGroupsInfo,
}

pub struct AttributeMeshReadView<'a> {
  pub attributes:
    SmallVec<[(&'a AttributeSemantic, AttributeAccessorReadView<'a>); MOST_COMMON_ATTRIBUTE_COUNT]>,
  pub indices: Option<(AttributeIndexFormat, AttributeAccessorReadView<'a>)>,
  pub mesh: &'a AttributesMesh,
}

impl<'a> std::ops::Deref for AttributeMeshReadView<'a> {
  type Target = AttributesMesh;

  fn deref(&self) -> &Self::Target {
    self.mesh
  }
}

impl<'a> AttributeMeshReadView<'a> {
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

pub struct PositionReader<'a> {
  position: &'a [Vec3<f32>],
}
impl<'a> IndexGet for PositionReader<'a> {
  type Output = Vec3<f32>;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    self.position.get(key).copied()
  }
}
pub type AttributeMeshShapeReadView<'a> = AttributeMeshCustomReadView<'a, PositionReader<'a>>;

pub struct FullReaderBase<'a> {
  pub keys: Vec<AttributeSemantic>,
  pub bytes: Vec<&'a [u8]>,
}

pub type AttributeMeshFullReadView<'a> = AttributeMeshCustomReadView<'a, FullReaderBase<'a>>;

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
  pub fn read(&self) -> AttributeMeshReadView {
    let attributes = self.attributes.iter().map(|(k, a)| (k, a.read())).collect();
    let indices = self.indices.as_ref().map(|(f, a)| (*f, a.read()));

    AttributeMeshReadView {
      attributes,
      indices,
      mesh: self,
    }
  }

  pub fn read_full(&self) -> AttributeMeshFullReadView {
    let inner = self.read();
    let reader = inner.create_full_read_view_base();
    // safety: the returned reference is origin from the buffer itself, no cyclic reference exists
    // the allocate temp buffer is immutable and has stable heap location.
    let reader = unsafe { std::mem::transmute(reader) };
    AttributeMeshFullReadView { inner, reader }
  }

  pub fn read_shape(&self) -> AttributeMeshShapeReadView {
    let inner = self.read();
    let position = inner.get_position();
    // safety: the returned reference is origin from the buffer itself, no cyclic reference exists
    let position = unsafe { std::mem::transmute(position) };
    AttributeMeshCustomReadView {
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
