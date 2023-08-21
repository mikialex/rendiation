use std::{any::TypeId, num::NonZeroU64};

use incremental::*;
use reactive_incremental::*;

use crate::{MeshGroupsInfo, PrimitiveTopology};

mod merge;
mod picking;

pub use merge::*;
pub use picking::*;

/// Vertex attribute semantic name.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum AttributeSemantic {
  /// XYZ vertex positions.
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

#[derive(Clone)]
pub struct ForeignAttributeKey {
  id: TypeId,
  pub implementation: Box<dyn AnyClone + Send + Sync>,
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
    self.id.partial_cmp(&other.id)
  }
}

impl std::hash::Hash for ForeignAttributeKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

#[derive(Clone)]
pub struct GeometryBufferInner {
  pub buffer: Vec<u8>,
}

clone_self_incremental!(GeometryBufferInner);

pub type GeometryBuffer = SharedIncrementalSignal<GeometryBufferInner>;

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
      buffer: self.buffer.read(),
      view: self,
    }
  }
}

pub struct UnTypedBufferViewReadView<'a> {
  buffer: SceneItemRefGuard<'a, GeometryBufferInner>,
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
  pub item_size: usize,
}

impl AttributeAccessor {
  pub fn create_owned<T: bytemuck::Pod>(input: Vec<T>, item_size: usize) -> Self {
    let buffer = bytemuck::cast_slice(&input).to_owned();

    let buffer = GeometryBufferInner { buffer };
    let buffer = buffer.into_ref();
    let view = UnTypedBufferView {
      buffer,
      range: Default::default(),
    };
    Self {
      view,
      byte_offset: 0,
      count: input.len(),
      item_size,
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
      size: NonZeroU64::new((self.count * self.item_size) as u64)
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

clone_self_incremental!(AttributesMesh);

#[derive(Clone)]
pub struct AttributesMesh {
  pub attributes: Vec<(AttributeSemantic, AttributeAccessor)>,
  pub indices: Option<(AttributeIndexFormat, AttributeAccessor)>,
  pub mode: PrimitiveTopology,
  pub groups: MeshGroupsInfo,
}

pub struct AttributeMeshReadView<'a> {
  pub attributes: Vec<(&'a AttributeSemantic, AttributeAccessorReadView<'a>)>,
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
  pub fn get_position(&self) -> &AttributeAccessorReadView {
    self
      .get_attribute(&AttributeSemantic::Positions)
      .expect("position attribute should always exist")
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

  pub fn get_attribute(&self, s: &AttributeSemantic) -> Option<&AttributeAccessor> {
    self.attributes.iter().find(|(k, _)| k == s).map(|r| &r.1)
  }
  pub fn get_position(&self) -> &AttributeAccessor {
    self
      .get_attribute(&AttributeSemantic::Positions)
      .expect("position attribute should always exist")
  }
}
