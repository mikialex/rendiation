use crate::*;

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

impl<'a> GPUConsumableMeshBuffer for AttributesMeshEntityReadView<'a> {
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

pub struct AttributesMeshEntityCustomReadView<'a, F> {
  pub inner: AttributesMeshEntityReadView<'a>,
  pub reader: F,
}

pub struct DynIndexView<'a> {
  fmt: AttributeIndexFormat,
  buffer: AttributeAccessorReadView<'a>,
}

impl<'a> IndexGet for DynIndexView<'a> {
  type Output = usize;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    match self.fmt {
      AttributeIndexFormat::Uint16 => self.buffer.visit_slice::<u16>()?.index_get(key)? as usize,
      AttributeIndexFormat::Uint32 => self.buffer.visit_slice::<u32>()?.index_get(key)? as usize,
    }
    .into()
  }
}

impl<'a, F: Clone> AttributesMeshEntityCustomReadView<'a, F> {
  pub fn as_abstract_mesh_read_view(
    &self,
  ) -> AttributesMeshEntityAbstractMeshReadView<F, DynIndexView<'a>> {
    AttributesMeshEntityAbstractMeshReadView {
      mode: self.inner.mode,
      vertices: self.reader.clone(),
      indices: self.indices.as_ref().map(|index| DynIndexView {
        fmt: index.0,
        buffer: index.1,
      }),
      count: self.inner.primitive_count(),
      draw_count: self.inner.draw_count(),
    }
  }
}

impl<'a, F> Deref for AttributesMeshEntityCustomReadView<'a, F> {
  type Target = AttributesMeshEntityReadView<'a>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'a, F> GPUConsumableMeshBuffer for AttributesMeshEntityCustomReadView<'a, F> {
  fn draw_count(&self) -> usize {
    self.mesh.draw_count()
  }
}

pub struct AttributesMeshEntityAbstractMeshReadView<T, I> {
  pub mode: PrimitiveTopology,
  pub vertices: T,
  pub indices: Option<I>,
  pub count: usize,
  pub draw_count: usize,
}

impl<T, I> GPUConsumableMeshBuffer for AttributesMeshEntityAbstractMeshReadView<T, I> {
  fn draw_count(&self) -> usize {
    self.draw_count
  }
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

impl<'a> IntersectAbleGroupedMesh for AttributesMeshEntityShapeReadView<'a> {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    let group = self.groups.get_group(group, self);
    self
      .as_abstract_mesh_read_view()
      .intersect_list(ray, conf, group, result);
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    let group = self.groups.get_group(group, self);
    self
      .as_abstract_mesh_read_view()
      .intersect_nearest(ray, conf, group)
  }
}
