use crate::*;

mod access;
mod merge;
mod picking;
mod semantic;

pub use access::*;
pub use merge::*;
pub use picking::*;
pub use semantic::*;

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
  pub buffer: Arc<Vec<u8>>,
  pub range: BufferViewRange,
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

    let view = UnTypedBufferView {
      buffer: Arc::new(buffer),
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

pub struct AttributesMeshData {
  pub attributes: Vec<(AttributeSemantic, Vec<u8>)>,
  pub indices: Option<(AttributeIndexFormat, Vec<u8>)>,
  pub mode: PrimitiveTopology,
  pub groups: MeshGroupsInfo,
}

impl AttributesMeshData {
  pub fn build(self) -> AttributesMesh {
    let attributes = self
      .attributes
      .into_iter()
      .map(|(s, buffer)| {
        let buffer = AttributeAccessor::create_owned(buffer, s.item_byte_size());
        (s, buffer)
      })
      .collect();

    AttributesMesh {
      attributes,
      indices: self.indices.map(|(fmt, buffer)| {
        let buffer = AttributeAccessor::create_owned(buffer, fmt.byte_size());
        (fmt, buffer)
      }),
      mode: self.mode,
      groups: self.groups,
    }
  }
}
