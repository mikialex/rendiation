use crate::*;

#[derive(Clone, Copy)]
pub struct LocalFrustumQueryRequest<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  pub local_frustum: &'a Frustum,
  pub helper: Option<&'a FrustumIntersectionTestHelper<f32>>,
  pub policy: ObjectTestPolicy,
  pub extra_screen_space_tolerance: f32,
  pub world_mat: &'a Mat4<f64>,
  pub camera_ctx: &'a CameraQueryCtx,
}

pub struct LocalFrustumSubPrimitiveQueryRequest<'a> {
  pub internal: LocalFrustumQueryRequest<'a>,
  pub results: &'a mut Vec<u32>,
}

impl<'a> std::ops::Deref for LocalFrustumSubPrimitiveQueryRequest<'a> {
  type Target = LocalFrustumQueryRequest<'a>;
  fn deref(&self) -> &Self::Target {
    &self.internal
  }
}

#[derive(Clone, Copy)]
pub struct LocalRayQueryRequest<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  pub local_ray: Ray3<f32>,
  pub local_tolerance: f32,
  pub extra_screen_space_tolerance: f32,
  pub world_mat: &'a Mat4<f64>,
  pub camera_ctx: &'a CameraQueryCtx,
}

pub struct LocalRayAllQueryRequest<'a> {
  pub internal: LocalRayQueryRequest<'a>,
  pub results: &'a mut Vec<MeshBufferHitPoint>,
}

impl<'a> std::ops::Deref for LocalRayAllQueryRequest<'a> {
  type Target = LocalRayQueryRequest<'a>;
  fn deref(&self) -> &Self::Target {
    &self.internal
  }
}

pub trait LocalModelPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>>;

  /// should return hit result in local space
  fn ray_query_local_nearest(&self, request: LocalRayQueryRequest) -> Option<MeshBufferHitPoint>;

  /// should return hit result in local space
  fn ray_query_local_all(&self, request: LocalRayAllQueryRequest) -> Option<()>;

  fn frustum_query_local(&self, request: LocalFrustumQueryRequest) -> Option<bool>;

  fn frustum_query_local_sub_primitives(
    &self,
    request: LocalFrustumSubPrimitiveQueryRequest,
  ) -> Option<()>;
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

  fn ray_query_local_nearest(&self, request: LocalRayQueryRequest) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) = provider.ray_query_local_nearest(request) {
        return Some(hit);
      }
    }
    None
  }

  fn ray_query_local_all(&self, request: LocalRayAllQueryRequest) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_local_all(LocalRayAllQueryRequest {
          results: request.results,
          ..request
        })
        .is_some()
      {
        return Some(());
      }
    }
    None
  }

  fn frustum_query_local(&self, request: LocalFrustumQueryRequest) -> Option<bool> {
    for provider in self {
      if let Some(r) = provider.frustum_query_local(request) {
        return Some(r);
      }
    }
    None
  }

  fn frustum_query_local_sub_primitives(
    &self,
    request: LocalFrustumSubPrimitiveQueryRequest,
  ) -> Option<()> {
    for provider in self {
      if let Some(r) =
        provider.frustum_query_local_sub_primitives(LocalFrustumSubPrimitiveQueryRequest {
          results: request.results,
          ..request
        })
      {
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

  fn ray_query_local_nearest(&self, request: LocalRayQueryRequest) -> Option<MeshBufferHitPoint> {
    // todo extra_screen_space_tolerance
    let config = MeshBufferIntersectConfig {
      tolerance_local: request.local_tolerance,
      triangle_face: FaceSide::Double,
    };

    *self
      .query_local_read_view(request.idx)?
      .ray_intersect_nearest(request.local_ray, &config)
  }

  /// should return hit result in local space
  fn ray_query_local_all(&self, request: LocalRayAllQueryRequest) -> Option<()> {
    // todo extra_screen_space_tolerance
    let config = MeshBufferIntersectConfig {
      tolerance_local: request.local_tolerance,
      triangle_face: FaceSide::Double,
    };
    self.query_local_read_view(request.idx)?.ray_intersect_all(
      request.local_ray,
      &config,
      request.results,
    );

    Some(())
  }

  fn frustum_query_local(&self, request: LocalFrustumQueryRequest) -> Option<bool> {
    let mesh = self.query_local_read_view(request.idx)?;

    // todo extra_screen_space_tolerance
    let r = frustum_test_abstract_mesh(&mesh, request.policy, |p| {
      frustum_test_primitive(&p, request.helper, request.local_frustum, request.policy)
    });

    Some(r)
  }

  fn frustum_query_local_sub_primitives(
    &self,
    request: LocalFrustumSubPrimitiveQueryRequest,
  ) -> Option<()> {
    let mesh = self.query_local_read_view(request.idx)?;

    // todo extra_screen_space_tolerance
    for (i, p) in mesh.primitive_iter().enumerate() {
      if frustum_test_primitive(&p, request.helper, request.local_frustum, request.policy) {
        request.results.push(i as u32);
      }
    }

    Some(())
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
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  f: &Frustum,
  policy: ObjectTestPolicy,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => match p {
      AttributeDynPrimitive::Points(point) => f.contains(&point.0),
      AttributeDynPrimitive::LineSegment(line) => {
        frustum_intersect_line_segment(helper, f, line.start, line.end)
      }
      AttributeDynPrimitive::Triangle(triangle) => frustum_test_tri(helper, f, triangle, policy),
    },
    ObjectTestPolicy::Contains => match p {
      AttributeDynPrimitive::Points(point) => f.contains(&point.0),
      AttributeDynPrimitive::LineSegment(line) => f.contains(&line.start) && f.contains(&line.end),
      AttributeDynPrimitive::Triangle(triangle) => frustum_test_tri(helper, f, triangle, policy),
    },
  }
}

#[inline(always)]
pub fn frustum_test_tri(
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  f: &Frustum,
  triangle: &Triangle3D,
  policy: ObjectTestPolicy,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => {
      frustum_intersect_triangle(helper, f, triangle.a, triangle.b, triangle.c)
    }
    ObjectTestPolicy::Contains => {
      f.contains(&triangle.a) && f.contains(&triangle.b) && f.contains(&triangle.c)
    }
  }
}

pub fn frustum_test_abstract_mesh_as_quad_all<G: AbstractMesh<Primitive = Triangle3D>>(
  mesh: &G,
  policy: ObjectTestPolicy,
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  frustum: &Frustum,
  mut collector: impl FnMut(usize),
) {
  for (i, [p1, p2]) in mesh.primitive_iter().array_chunks::<2>().enumerate() {
    let r = match policy {
      ObjectTestPolicy::Intersect => {
        frustum_test_tri(helper, frustum, &p1, policy)
          || frustum_test_tri(helper, frustum, &p2, policy)
      }
      ObjectTestPolicy::Contains => {
        frustum_test_tri(helper, frustum, &p1, policy)
          && frustum_test_tri(helper, frustum, &p2, policy)
      }
    };
    if r {
      collector(i);
    }
  }
}
