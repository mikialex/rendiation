use std::num::NonZeroU64;

use futures::StreamExt;
use reactive::once_forever_pending;
use reactive::{PollUtils, SignalStreamExt};
use rendiation_geometry::Box3;
use rendiation_geometry::SpaceBounding;
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

// todo should use macro
pub fn register_core_mesh_features<T>()
where
  T: AsRef<dyn IntersectAbleGroupedMesh>
    + AsMut<dyn IntersectAbleGroupedMesh>
    + AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    // + AsRef<dyn WatchableSceneMeshLocalBounding>
    // + AsMut<dyn WatchableSceneMeshLocalBounding>
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

/// Vertex attribute semantic name.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
  // pub byte_stride: Option<usize>, todo
}

impl UnTypedBufferView {
  pub fn visit_bytes<R>(
    &self,
    view_byte_offset: usize,
    visitor: impl FnOnce(&[u8]) -> R,
  ) -> Option<R> {
    let buffer = self.buffer.read();
    let byte_slice = buffer.buffer.as_slice();
    let offset = self.range.offset as usize + view_byte_offset;

    let byte_slice = if let Some(byte_size) = self.range.size {
      let byte_size = Into::<u64>::into(byte_size) as usize;
      byte_slice.get(offset..offset + byte_size)
    } else {
      byte_slice.get(offset..)
    }?;

    Some(visitor(byte_slice))
  }

  pub fn visit_slice<T: bytemuck::Pod, R>(
    &self,
    view_byte_offset: usize,
    typed_count: usize,
    visitor: impl FnOnce(&[T]) -> R,
  ) -> Option<R> {
    self
      .visit_bytes(view_byte_offset, |byte_slice| {
        let cast_slice = bytemuck::try_cast_slice(byte_slice).ok()?;
        let slice = cast_slice.get(0..typed_count)?;
        Some(visitor(slice))
      })
      .flatten()
  }

  pub fn get<T: bytemuck::Pod>(
    &self,
    view_byte_offset: usize,
    typed_count: usize,
    index: usize,
  ) -> Option<T> {
    self
      .visit_slice(view_byte_offset, typed_count, |slice| {
        slice.get(index).cloned()
      })
      .flatten()
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
  pub fn visit_bytes<R>(&self, visitor: impl FnOnce(&[u8]) -> R) -> Option<R> {
    self.view.visit_bytes(self.byte_offset, visitor)
  }

  pub fn visit_slice<T: bytemuck::Pod, R>(&self, visitor: impl FnOnce(&[T]) -> R) -> Option<R> {
    self.view.visit_slice(self.byte_offset, self.count, visitor)
  }
  pub fn get<T: bytemuck::Pod>(&self, index: usize) -> Option<T> {
    self.view.get(self.byte_offset, self.count, index)
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
  pub groups: MeshGroupsInfo,
}

impl AttributesMesh {
  pub fn get_attribute(&self, s: AttributeSemantic) -> Option<&AttributeAccessor> {
    self.attributes.iter().find(|(k, _)| *k == s).map(|r| &r.1)
  }
  pub fn get_position(&self) -> &AttributeAccessor {
    self
      .get_attribute(AttributeSemantic::Positions)
      .expect("position attribute should always exist")
  }
}

pub trait WatchableSceneMeshLocalBounding {
  fn build_local_bound_stream(&self) -> Box<dyn Stream<Item = Option<Box3>> + Unpin>;
}
define_dyn_trait_downcaster_static!(WatchableSceneMeshLocalBounding);

impl WatchableSceneMeshLocalBounding for SceneMeshType {
  fn build_local_bound_stream(&self) -> Box<dyn Stream<Item = Option<Box3>> + Unpin> {
    match self {
      SceneMeshType::AttributesMesh(mesh) => {
        let st = mesh
          .single_listen_by(any_change)
          .filter_map_sync(mesh.defer_weak())
          .map(|mesh| {
            let mesh = mesh.read();
            let local: Box3 = mesh.primitive_iter().map(|p| p.to_bounding()).collect();
            local.into()
          });
        Box::new(st) as Box<dyn Stream<Item = Option<Box3>> + Unpin>
      }
      SceneMeshType::TransformInstanced(mesh) => {
        let st = mesh
          .single_listen_by(any_change)
          .filter_map_sync(mesh.defer_weak())
          .map(|mesh| {
            let mesh = mesh.read();

            let inner_bounding = mesh
              .mesh
              .build_local_bound_stream()
              .consume_self_get_next()
              .unwrap();

            inner_bounding.map(|inner_bounding| {
              mesh
                .transforms
                .iter()
                .map(|mat| inner_bounding.apply_matrix_into(*mat))
                .collect::<Box3>()
            })
          });
        Box::new(st)
      }
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = get_dyn_trait_downcaster_static!(WatchableSceneMeshLocalBounding)
          .downcast_ref(mesh.as_ref())
        {
          mesh.build_local_bound_stream()
        } else {
          Box::new(once_forever_pending(None))
        }
      }
    }
  }
}
