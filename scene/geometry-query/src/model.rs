use crate::*;

pub trait LocalModelPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>>;

  /// should return hit result in local space
  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint>;

  /// should return hit result in local space
  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<()>;

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_frustum: &Frustum,
    policy: ObjectTestPolicy,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<bool>;
}

impl LocalModelPicker for Vec<Box<dyn LocalModelPicker>> {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    for provider in self {
      if let Some(hit) = provider.bounding_enlarge_tolerance(idx) {
        return Some(hit);
      }
    }
    None
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) =
        provider.ray_query_local_nearest(idx, local_ray, local_tolerance, world_mat, camera_ctx)
      {
        return Some(hit);
      }
    }
    None
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_local_all(
          idx,
          local_ray,
          local_tolerance,
          results,
          world_mat,
          camera_ctx,
        )
        .is_some()
      {
        return Some(());
      }
    }
    None
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    frustum: &Frustum,
    policy: ObjectTestPolicy,
    world_mat: &Mat4<f64>,
    camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    for provider in self {
      if let Some(r) = provider.frustum_query_local(idx, frustum, policy, world_mat, camera_ctx) {
        return Some(r);
      }
    }
    None
  }
}

pub fn use_attribute_mesh_picker<Cx: DBHookCxLike>(cx: &mut Cx) -> Option<AttributeMeshPicker> {
  let mesh_vertex_refs = cx
    .use_db_rev_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .use_assure_result(cx);

  cx.when_resolve_stage(|| AttributeMeshPicker {
    model_access_std_model: read_global_db_foreign_key(),
    std_model_access_mesh: read_global_db_foreign_key(),
    mesh_vertex_refs: mesh_vertex_refs.expect_resolve_stage().into_boxed_multi(),
    semantic: read_global_db_component(),
    vertex_buffer: SceneBufferViewReadView::new_from_global(),
    index_buffer: SceneBufferViewReadView::new_from_global(),
    mesh_topology: read_global_db_component(),
    buffer: read_global_db_component(),
    pick_line_tolerance: IntersectTolerance::new(1.0, ToleranceType::ScreenSpace),
    pick_point_tolerance: IntersectTolerance::new(1.0, ToleranceType::ScreenSpace),
  })
}

pub struct AttributeMeshPicker {
  pub model_access_std_model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  pub std_model_access_mesh: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  pub mesh_vertex_refs: BoxedDynMultiQuery<RawEntityHandle, RawEntityHandle>,
  pub vertex_buffer: SceneBufferViewReadView<AttributeVertexRef>,
  pub index_buffer: SceneBufferViewReadView<AttributeIndexRef>,
  pub semantic: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,
  pub mesh_topology: ComponentReadView<AttributesMeshEntityTopology>,
  pub buffer: ComponentReadView<BufferEntityData>,
  pub pick_line_tolerance: IntersectTolerance,
  pub pick_point_tolerance: IntersectTolerance,
}

struct AttributeFastPickView<'a> {
  buffer: &'a [Vec3<f32>],
}

impl IndexGet for AttributeFastPickView<'_> {
  type Output = Vec3<f32>;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    self.buffer.get(key).copied()
  }
}

impl AttributeMeshPicker {
  fn query_local_read_view(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<AttributesMeshEntityAbstractMeshReadView<AttributeFastPickView<'_>, DynIndexRef<'_>>>
  {
    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;

    let mode = self.mesh_topology.get_value(mesh)?;

    let mut position: Option<&[Vec3<f32>]> = None;
    let mut count = 0;
    for att in self.mesh_vertex_refs.access_multi(&mesh.into_raw())? {
      let att = unsafe { EntityHandle::from_raw(att) };
      if let AttributeSemantic::Positions = self.semantic.get_value(att)? {
        let p = self
          .vertex_buffer
          .read_view_slice::<Vec3<f32>>(att, &self.buffer)?;
        position = p.into();
        count = p.len();
        break;
      }
    }
    let position = AttributeFastPickView { buffer: position? };

    let index =
      self
        .index_buffer
        .read_view_bytes(mesh, &self.buffer)
        .map(|(buffer, index_count)| {
          count = index_count as usize;
          let byte_per_item = buffer.len() / index_count as usize;
          if byte_per_item == 4 {
            let index: &[u32] = cast_slice(buffer);
            DynIndexRef::Uint32(index)
          } else {
            let index: &[u16] = cast_slice(buffer);
            DynIndexRef::Uint16(index)
          }
        });

    AttributesMeshEntityAbstractMeshReadView {
      mode,
      vertices: position,
      indices: index,
      count: count / mode.stride(),
    }
    .into()
  }
}

impl LocalModelPicker for AttributeMeshPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;
    let topo = self.mesh_topology.get_value(mesh)?;
    let tor = match topo {
      MeshPrimitiveTopology::PointList => self.pick_point_tolerance,
      MeshPrimitiveTopology::LineList => self.pick_line_tolerance,
      MeshPrimitiveTopology::LineStrip => self.pick_line_tolerance,
      _ => return Some(None),
    };
    Some(Some(tor))
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    let config = MeshBufferIntersectConfig {
      tolerance_local: local_tolerance,
      triangle_face: FaceSide::Double,
    };

    *self
      .query_local_read_view(idx)?
      .ray_intersect_nearest(local_ray, &config)
  }

  /// should return hit result in local space
  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    let config = MeshBufferIntersectConfig {
      tolerance_local: local_tolerance,
      triangle_face: FaceSide::Double,
    };
    self
      .query_local_read_view(idx)?
      .ray_intersect_all(local_ray, &config, results);

    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    frustum: &Frustum,
    policy: ObjectTestPolicy,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    let mesh = self.query_local_read_view(idx)?;

    let r = frustum_test_abstract_mesh(&mesh, policy, |p| {
      frustum_test_primitive(&p, &frustum, policy)
    });

    Some(r)
  }
}

pub fn frustum_test_abstract_mesh<G: AbstractMesh>(
  mesh: &G,
  policy: ObjectTestPolicy,
  tester: impl Fn(G::Primitive) -> bool,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => mesh.primitive_iter().any(tester),
    ObjectTestPolicy::Contains => mesh.primitive_iter().all(tester),
  }
}

fn frustum_test_primitive(
  p: &AttributeDynPrimitive,
  f: &Frustum,
  policy: ObjectTestPolicy,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => match p {
      AttributeDynPrimitive::Points(point) => f.contains(&point.0),
      AttributeDynPrimitive::LineSegment(line) => f.contains(&line.start) || f.contains(&line.end),
      AttributeDynPrimitive::Triangle(triangle) => frustum_test_tri(f, &triangle, policy),
    },
    ObjectTestPolicy::Contains => match p {
      AttributeDynPrimitive::Points(point) => f.contains(&point.0),
      AttributeDynPrimitive::LineSegment(line) => f.contains(&line.start) && f.contains(&line.end),
      AttributeDynPrimitive::Triangle(triangle) => frustum_test_tri(f, &triangle, policy),
    },
  }
}

#[inline(always)]
pub fn frustum_test_tri(f: &Frustum, triangle: &Triangle3D, policy: ObjectTestPolicy) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => {
      // todo, this is wrong
      f.contains(&triangle.a) || f.contains(&triangle.b) || f.contains(&triangle.c)
    }
    ObjectTestPolicy::Contains => {
      f.contains(&triangle.a) && f.contains(&triangle.b) && f.contains(&triangle.c)
    }
  }
}
