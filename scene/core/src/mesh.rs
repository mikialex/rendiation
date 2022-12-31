use std::num::NonZeroU64;

use rendiation_geometry::{Box3, OptionalNearest, Ray3};
use rendiation_renderable_mesh::*;

use crate::*;

pub type SceneMesh = SceneItemRef<SceneMeshType>;

#[non_exhaustive]
#[derive(Clone)]
pub enum SceneMeshType {
  AttributesMesh(SceneItemRef<AttributesMesh>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

impl SceneMeshType {
  pub fn compute_local_bound(&self) -> Option<Box3> {
    None
  }
}

clone_self_incremental!(SceneMeshType);

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
pub struct TypedBufferView {
  pub buffer: GeometryBuffer,
  pub range: BufferViewRange,
}

#[derive(Clone)]
pub struct AttributeAccessor {
  pub view: TypedBufferView,
  pub start: usize,
  pub count: usize,
  pub stride: usize,
}

impl AttributeAccessor {
  pub fn compute_gpu_buffer_range(&self) -> BufferViewRange {
    let inner_offset = self.view.range.offset;
    BufferViewRange {
      offset: inner_offset + (self.start * self.stride) as u64,
      size: NonZeroU64::new(inner_offset + (self.count * self.stride) as u64)
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

impl IntersectAbleGroupedMesh for AttributesMesh {
  fn intersect_list(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _result: &mut MeshBufferHitList,
    _group: MeshDrawGroup,
  ) {
  }

  fn intersect_nearest(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    OptionalNearest::none()
  }
}
