use rendiation_geometry::*;
use rendiation_renderable_mesh::*;

use crate::*;

impl<T: IntersectAbleGroupedMesh + IncrementalBase> IntersectAbleGroupedMesh for SceneItemRef<T> {
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
  for SceneItemRef<T>
{
  fn as_ref(&self) -> &(dyn IntersectAbleGroupedMesh + 'static) {
    self
  }
}
impl<T: IntersectAbleGroupedMesh + IncrementalBase> AsMut<dyn IntersectAbleGroupedMesh>
  for SceneItemRef<T>
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

/// this is slow, but not bloat the binary size.
impl AbstractMesh for AttributesMesh {
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

    #[rustfmt::skip]
     if let Some((fmt, index)) = &self.indices {
      self.get_position().visit_slice::<Vec3<f32>, Option<Self::Primitive>>(|position|{
        match fmt {
          AttributeIndexFormat::Uint16 => {
            index.visit_slice::<u16, Option<Self::Primitive>>(|index|{
              match self.mode{
                PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
              }.into()
            }).flatten()
          },
          AttributeIndexFormat::Uint32 => {
            index.visit_slice::<u32, Option<Self::Primitive>>(|index|{
              match self.mode{
                PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
                PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&index, read_index)?.f_filter_map(|id|position.get(id as usize).copied())?),
              }.into()
            }).flatten()
          },
        }
      }).flatten()
    } else {
      self.get_position().visit_slice::<Vec3<f32>, Option<Self::Primitive>>(|position|{
        match self.mode{
          PrimitiveTopology::PointList => AttributeDynPrimitive::Points(Point::from_data(&position, read_index)?),
          PrimitiveTopology::LineList => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&position, read_index)?),
          PrimitiveTopology::LineStrip => AttributeDynPrimitive::LineSegment(LineSegment::from_data(&position, read_index)?),
          PrimitiveTopology::TriangleList => AttributeDynPrimitive::Triangle(Triangle::from_data(&position, read_index)?),
          PrimitiveTopology::TriangleStrip => AttributeDynPrimitive::Triangle(Triangle::from_data(&position, read_index)?),
        }.into()
      }).flatten()
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

impl IntersectAbleGroupedMesh for AttributesMesh {
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

impl IntersectAbleGroupedMesh for TransformInstancedSceneMesh {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self.transforms.iter().for_each(|mat| {
      let world_inv = mat.inverse_or_identity();
      let local_ray = ray.clone().apply_matrix_into(world_inv);
      self
        .mesh
        .intersect_list_by_group(local_ray, conf, result, group)
    })
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self
      .transforms
      .iter()
      .fold(OptionalNearest::none(), |mut pre, mat| {
        let world_inv = mat.inverse_or_identity();
        let local_ray = ray.clone().apply_matrix_into(world_inv);
        let r = self.mesh.intersect_nearest_by_group(local_ray, conf, group);
        *pre.refresh_nearest(r)
      })
  }
}

impl IntersectAbleGroupedMesh for SceneMeshType {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    match self {
      SceneMeshType::AttributesMesh(mesh) => mesh
        .read()
        .intersect_list_by_group(ray, conf, result, group),
      SceneMeshType::TransformInstanced(mesh) => mesh
        .read()
        .intersect_list_by_group(ray, conf, result, group),
      SceneMeshType::Foreign(mesh) => {
        if let Some(pickable) =
          get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh).downcast_ref(mesh.as_ref())
        {
          pickable.intersect_list_by_group(ray, conf, result, group)
        }
      }
    }
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    match self {
      SceneMeshType::AttributesMesh(mesh) => {
        mesh.read().intersect_nearest_by_group(ray, conf, group)
      }
      SceneMeshType::TransformInstanced(mesh) => {
        mesh.read().intersect_nearest_by_group(ray, conf, group)
      }
      SceneMeshType::Foreign(mesh) => {
        if let Some(pickable) =
          get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh).downcast_ref(mesh.as_ref())
        {
          pickable.intersect_nearest_by_group(ray, conf, group)
        } else {
          OptionalNearest::none()
        }
      }
    }
  }
}
