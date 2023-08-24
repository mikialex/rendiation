use incremental::*;
use reactive_incremental::*;
use rendiation_algebra::*;
use rendiation_geometry::*;

use crate::*;

impl<T: IntersectAbleGroupedMesh + IncrementalBase> IntersectAbleGroupedMesh
  for SharedIncrementalSignal<T>
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
  for SharedIncrementalSignal<T>
{
  fn as_ref(&self) -> &(dyn IntersectAbleGroupedMesh + 'static) {
    self
  }
}
impl<T: IntersectAbleGroupedMesh + IncrementalBase> AsMut<dyn IntersectAbleGroupedMesh>
  for SharedIncrementalSignal<T>
{
  fn as_mut(&mut self) -> &mut (dyn IntersectAbleGroupedMesh + 'static) {
    self
  }
}

pub enum AttributeDynPrimitive {
  Points(Point<Vec3<f32>>),
  LineSegment(LineSegment<Vec3<f32>>),
  Triangle(Triangle<Vec3<f32>>),
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

/// we can not impl AbstractMesh for AttributeMesh because it contains interior mutability
///
/// this is slow, but not bloat the binary size.
impl<'a> AbstractMesh for AttributeMeshReadView<'a> {
  type Primitive = AttributeDynPrimitive;

  fn primitive_count(&self) -> usize {
    let count = if let Some((_, index)) = &self.indices {
      index.count
    } else {
      self.get_position().count
    };

    (count + self.mode.step() - self.mode.stride()) / self.mode.step()
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let read_index = self.mode.step() * primitive_index;
    let position = self.get_position().visit_slice::<Vec3<f32>>()?;

    #[rustfmt::skip]
     if let Some((fmt, index)) = &self.indices {
      match fmt {
        AttributeIndexFormat::Uint16 => {
          let index = index.visit_slice::<u16>()?;
          match self.mode{
            PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
          }.into()
        },
        AttributeIndexFormat::Uint32 => {
          let index = index.visit_slice::<u32>()?;
          match self.mode{
            PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
            PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
          }.into()
        },
      }
    } else {
      match self.mode{
        PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&position, read_index)?),
        PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&position, read_index)?),
        PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&position, read_index)?),
        PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&position, read_index)?),
        PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&position, read_index)?),
      }.into()
    }
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

impl<'a> IntersectAbleGroupedMesh for AttributeMeshReadView<'a> {
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
