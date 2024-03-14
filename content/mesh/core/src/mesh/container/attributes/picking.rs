use std::ops::Deref;

use incremental::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;

use crate::*;

impl<T: IntersectAbleGroupedMesh + IncrementalBase> IntersectAbleGroupedMesh
  for IncrementalSignalPtr<T>
{
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self
      .read()
      .intersect_list_by_group(ray, conf, result, group)
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self.read().intersect_nearest_by_group(ray, conf, group)
  }
}
impl<T: IntersectAbleGroupedMesh + IncrementalBase> AsRef<dyn IntersectAbleGroupedMesh>
  for IncrementalSignalPtr<T>
{
  fn as_ref(&self) -> &(dyn IntersectAbleGroupedMesh + 'static) {
    self
  }
}
impl<T: IntersectAbleGroupedMesh + IncrementalBase> AsMut<dyn IntersectAbleGroupedMesh>
  for IncrementalSignalPtr<T>
{
  fn as_mut(&mut self) -> &mut (dyn IntersectAbleGroupedMesh + 'static) {
    self
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

impl IntersectAble<Ray3, OptionalNearest<HitPoint3D>, MeshBufferIntersectConfig>
  for AttributeDynPrimitive
{
  fn intersect(
    &self,
    other: &Ray3,
    param: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<HitPoint3D> {
    match self {
      AttributeDynPrimitive::Points(v) => v.intersect(other, param),
      AttributeDynPrimitive::LineSegment(v) => v.intersect(other, param),
      AttributeDynPrimitive::Triangle(v) => v.intersect(other, param),
    }
  }
}

impl SpaceBounding<f32, Box3, 3> for AttributeDynPrimitive {
  fn to_bounding(&self) -> Box3 {
    match self {
      AttributeDynPrimitive::Points(v) => v.to_bounding(),
      AttributeDynPrimitive::LineSegment(v) => v.to_bounding(),
      AttributeDynPrimitive::Triangle(v) => v.to_bounding(),
    }
  }
}

impl<'a> GPUConsumableMeshBuffer for AttributeMeshReadView<'a> {
  fn draw_count(&self) -> usize {
    self.mesh.draw_count()
  }
}

impl GPUConsumableMeshBuffer for AttributesMesh {
  fn draw_count(&self) -> usize {
    if let Some((_, index)) = &self.indices {
      index.count
    } else {
      let position = self.get_position();
      position.count
    }
  }
}

pub struct AttributeMeshCustomReadView<'a, F> {
  pub inner: AttributeMeshReadView<'a>,
  pub reader: F,
}

impl<'a, F> Deref for AttributeMeshCustomReadView<'a, F> {
  type Target = AttributeMeshReadView<'a>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'a, F> GPUConsumableMeshBuffer for AttributeMeshCustomReadView<'a, F> {
  fn draw_count(&self) -> usize {
    self.mesh.draw_count()
  }
}

/// we can not impl AbstractMesh for AttributeMesh because it contains interior mutability.
///
/// this is slow, but not bloat the binary size.
impl<'a, F, V> AbstractMesh for AttributeMeshCustomReadView<'a, F>
where
  F: IndexGet<Output = V>,
  V: Copy,
{
  type Primitive = AttributeDynPrimitive<V>;

  fn primitive_count(&self) -> usize {
    let count = if let Some((_, index)) = &self.indices {
      index.count
    } else {
      self.get_position().len()
    };

    (count + self.mode.step() - self.mode.stride()) / self.mode.step()
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let read_index = self.mode.step() * primitive_index;

    #[rustfmt::skip]
     if let Some((fmt, index)) = &self.indices {
      match fmt {
        AttributeIndexFormat::Uint16 => {
          let index = index.visit_slice::<u16>()?;
          match self.mode{
            PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
          }.into()
        },
        AttributeIndexFormat::Uint32 => {
          let index = index.visit_slice::<u32>()?;
          match self.mode{
            PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
            PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|self.reader.index_get(id as usize))?),
          }.into()
        },
      }
    } else {
      match self.mode{
        PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&self.reader, read_index)?),
        PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&self.reader, read_index)?),
        PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&self.reader, read_index)?),
        PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&self.reader, read_index)?),
        PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&self.reader, read_index)?),
      }.into()
    }
  }
}

impl<'a> IntersectAbleGroupedMesh for AttributeMeshShapeReadView<'a> {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    let group = self.groups.get_group(group, self);
    self.intersect_list(ray, conf, group, result);
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    let group = self.groups.get_group(group, self);
    self.intersect_nearest(ray, conf, group)
  }
}
