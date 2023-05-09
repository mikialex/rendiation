use std::num::NonZeroU64;

use reactive::once_forever_pending;
use rendiation_geometry::Box3;
use rendiation_renderable_mesh::*;

use crate::*;

#[non_exhaustive]
#[derive(Clone)]
pub enum SceneMeshType {
  AttributesMesh(SceneItemRef<AttributesMesh>),
  TransformInstanced(SceneItemRef<TransformInstancedSceneMesh>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

clone_self_incremental!(SceneMeshType);

pub fn register_core_mesh_features<T>()
where
  T: AsRef<dyn IntersectAbleGroupedMesh>
    + AsMut<dyn IntersectAbleGroupedMesh>
    + AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    + 'static,
{
  get_dyn_trait_downcaster_static!(GlobalIdentified).register::<T>();
  get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh).register::<T>();
}

impl SceneMeshType {
  pub fn guid(&self) -> Option<usize> {
    match self {
      Self::AttributesMesh(m) => m.guid(),
      Self::TransformInstanced(m) => m.guid(),
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(GlobalIdentified)
        .downcast_ref(m.as_ref())?
        .guid(),
    }
    .into()
  }
}

#[derive(Clone)]
pub struct TransformInstancedSceneMesh {
  pub mesh: SceneMeshType,
  pub transforms: Vec<Mat4<f32>>,
}
clone_self_incremental!(TransformInstancedSceneMesh);

impl SceneMeshType {
  pub fn build_local_bound_stream(&self) -> impl Stream<Item = Option<Box3>> {
    once_forever_pending(None)
    // match self {
    //   SceneMeshType::AttributesMesh(_) => todo!(),
    //   SceneMeshType::TransformInstanced(_) => todo!(),
    //   SceneMeshType::Foreign(_) => once_forever_pending(None),
    // }
  }
}

/// Vertex attribute semantic name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
}

#[derive(Clone)]
pub struct GeometryBufferInner {
  pub buffer: Vec<u8>,
}

clone_self_incremental!(GeometryBufferInner);

pub type GeometryBuffer = SceneItemRef<GeometryBufferInner>;

#[derive(Debug, Copy, Clone, Default)]
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
}

impl UnTypedBufferView {
  pub fn visit_slice<T: bytemuck::Pod, R>(
    &self,
    range: std::ops::Range<usize>,
    visitor: impl FnOnce(&[T]) -> R,
  ) -> Option<R> {
    let buffer = self.buffer.read();
    let byte_slice = buffer.buffer.as_slice();
    let offset = self.range.offset as usize;
    let byte_slice = if let Some(byte_size) = self.range.size {
      let byte_size = Into::<u64>::into(byte_size) as usize;
      byte_slice.get(offset..offset + byte_size)
    } else {
      byte_slice.get(offset..)
    }?;

    let cast_slice = bytemuck::try_cast_slice(byte_slice).ok()?;
    let slice = cast_slice.get(range)?;
    Some(visitor(slice))
  }

  pub fn get<T: bytemuck::Pod>(
    &self,
    sub_range: std::ops::Range<usize>,
    index: usize,
  ) -> Option<T> {
    self
      .visit_slice(sub_range, |slice| slice.get(index).cloned())
      .flatten()
  }
}

#[derive(Clone)]
pub struct AttributeAccessor {
  pub view: UnTypedBufferView,
  pub start: usize,
  pub count: usize,
  pub item_size: usize,
}

impl AttributeAccessor {
  pub fn visit_slice<T: bytemuck::Pod, R>(&self, visitor: impl FnOnce(&[T]) -> R) -> Option<R> {
    self.view.visit_slice(self.start..self.count, visitor)
  }
  pub fn get<T: bytemuck::Pod>(&self, index: usize) -> Option<T> {
    self.view.get(self.start..self.count, index)
  }
}

impl AttributeAccessor {
  pub fn compute_gpu_buffer_range(&self) -> BufferViewRange {
    let inner_offset = self.view.range.offset;
    BufferViewRange {
      offset: inner_offset + (self.start * self.item_size) as u64,
      size: NonZeroU64::new((self.count * self.item_size) as u64)
        .unwrap() // safe
        .into(),
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum IndexFormat {
  /// Indices are 16 bit unsigned integers.
  Uint16 = 0,
  /// Indices are 32 bit unsigned integers.
  Uint32 = 1,
}

clone_self_incremental!(AttributesMesh);

#[derive(Clone)]
pub struct AttributesMesh {
  pub attributes: Vec<(AttributeSemantic, AttributeAccessor)>,
  pub indices: Option<(IndexFormat, AttributeAccessor)>,
  pub mode: PrimitiveTopology,
}

impl AttributesMesh {
  pub fn get_position(&self) -> &AttributeAccessor {
    let (_, position) = self
      .attributes
      .iter()
      .find(|(k, _)| *k == AttributeSemantic::Positions)
      .expect("position attribute should always exist");
    position
  }
}
