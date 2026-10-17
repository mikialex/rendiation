use crate::*;

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
      if let Some((buffer, index_count)) = self.index_buffer.read_view_bytes(mesh, &self.buffer) {
        count = index_count as usize;
        if count == 0 {
          return None;
        }
        let index = match buffer.len() / count {
          4 => DynIndexRef::Uint32(try_cast_slice(buffer).ok()?),
          2 => DynIndexRef::Uint16(try_cast_slice(buffer).ok()?),
          _ => return None,
        };
        Some(index)
      } else {
        None
      };

    // strip topology shares vertices between adjacent primitives
    let primitive_count = (count + mode.step()).saturating_sub(mode.stride()) / mode.step();

    AttributesMeshEntityAbstractMeshReadView {
      mode,
      vertices: position,
      indices: index,
      count: primitive_count,
    }
    .into()
  }
}

impl LocalModelPicker for AttributeMeshPicker {
  fn is_applicable(&self, idx: EntityHandle<SceneModelEntity>) -> bool {
    self
      .model_access_std_model
      .get(idx)
      .and_then(|model| self.std_model_access_mesh.get(model))
      .is_some()
  }

  fn tolerance(&self, idx: EntityHandle<SceneModelEntity>) -> Option<IntersectTolerance> {
    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;
    match self.mesh_topology.get_value(mesh)? {
      MeshPrimitiveTopology::PointList => Some(self.pick_point_tolerance),
      MeshPrimitiveTopology::LineList | MeshPrimitiveTopology::LineStrip => {
        Some(self.pick_line_tolerance)
      }
      _ => None,
    }
  }

  fn primitive_count(&self, idx: EntityHandle<SceneModelEntity>) -> u32 {
    self
      .query_local_read_view(idx)
      .map(|mesh| mesh.primitive_count() as u32)
      .unwrap_or(0)
  }

  fn ray_query_local(&self, req: LocalRayQuery, v: &mut dyn LocalRayHitVisitor) -> ControlFlow<()> {
    let Some(mesh) = self.query_local_read_view(req.idx) else {
      return ControlFlow::Continue(());
    };
    let config = MeshBufferIntersectConfig {
      tolerance_local: req.local_tolerance,
      triangle_face: FaceSide::Double,
    };
    let hits = mesh.ray_intersect_iter(req.local_ray, &config);
    visit_mesh_ray_hits(&req.local_ray, hits, |i| i as u32, v)
  }

  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()> {
    let Some(mesh) = self.query_local_read_view(req.idx) else {
      return ControlFlow::Continue(());
    };
    // todo extra_screen_space_tolerance and line point tolerance
    visit_frustum_primitives(
      mesh.primitive_iter(),
      |p| frustum_test_primitive(p, req.helper, req.local_frustum, req.policy),
      v,
    )
  }
}

fn frustum_test_primitive(
  p: &AttributeDynPrimitive,
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
  f: &Frustum,
  policy: ObjectTestPolicy,
) -> bool {
  match p {
    AttributeDynPrimitive::Points(point) => f.contains(&point.0),
    AttributeDynPrimitive::LineSegment(line) => frustum_test_line_segment(helper, f, line, policy),
    AttributeDynPrimitive::Triangle(triangle) => frustum_test_tri(helper, f, triangle, policy),
  }
}
