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

type AttributeMeshReadView<'a> =
  AttributesMeshEntityAbstractMeshReadView<AttributeFastPickView<'a>, DynIndexRef<'a>>;

impl AttributeMeshPicker {
  fn read_mesh_view(
    &self,
    mesh: EntityHandle<AttributesMeshEntity>,
    mode: MeshPrimitiveTopology,
  ) -> Result<AttributeMeshReadView<'_>, LocalModelPickDataError> {
    let err = LocalModelPickDataError;

    let mut position = None;
    let vertex_refs = self
      .mesh_vertex_refs
      .access_multi(&mesh.into_raw())
      .ok_or(err("vertex attributes"))?;
    for att in vertex_refs {
      let att = unsafe { EntityHandle::from_raw(att) };
      let semantic = self
        .semantic
        .get_value(att)
        .ok_or(err("vertex attribute semantic"))?;
      if let AttributeSemantic::Positions = semantic {
        let p = self
          .vertex_buffer
          .read_view_slice::<Vec3<f32>>(att, &self.buffer)
          .ok_or(err("position buffer"))?;
        position = p.into();
        break;
      }
    }
    let position: &[Vec3<f32>] = position.ok_or(err("position attribute"))?;
    let mut count = position.len();
    let position = AttributeFastPickView { buffer: position };

    let index = match self.index_buffer.read_view_bytes(mesh, &self.buffer) {
      Some((_, 0)) => {
        count = 0;
        None
      }
      Some((buffer, index_count)) => {
        count = index_count as usize;
        let index = match buffer.len() / count {
          4 => DynIndexRef::Uint32(try_cast_slice(buffer).map_err(|_| err("index buffer"))?),
          2 => DynIndexRef::Uint16(try_cast_slice(buffer).map_err(|_| err("index buffer"))?),
          _ => return Err(err("index format")),
        };
        Some(index)
      }
      None => None,
    };

    // strip topology shares vertices between adjacent primitives
    let primitive_count = (count + mode.step()).saturating_sub(mode.stride()) / mode.step();

    Ok(AttributesMeshEntityAbstractMeshReadView {
      mode,
      vertices: position,
      indices: index,
      count: primitive_count,
    })
  }
}

impl LocalModelPickerResolver for AttributeMeshPicker {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;

    let Some(mode) = self.mesh_topology.get_value(mesh) else {
      return Some(Err(LocalModelPickDataError("mesh topology")));
    };
    let tolerance = match mode {
      MeshPrimitiveTopology::PointList => Some(self.pick_point_tolerance),
      MeshPrimitiveTopology::LineList | MeshPrimitiveTopology::LineStrip => {
        Some(self.pick_line_tolerance)
      }
      _ => None,
    };
    let resolved = ResolvedAttributeMeshPicker {
      picker: self,
      mesh,
      mode,
      tolerance,
    };
    Some(Ok(Box::new_in(resolved, alloc)))
  }
}

/// the mesh view is not created in the resolve, because the model may be culled by the bounding
struct ResolvedAttributeMeshPicker<'a> {
  picker: &'a AttributeMeshPicker,
  mesh: EntityHandle<AttributesMeshEntity>,
  mode: MeshPrimitiveTopology,
  tolerance: Option<IntersectTolerance>,
}

impl ResolvedAttributeMeshPicker<'_> {
  fn mesh_view(&self) -> Result<AttributeMeshReadView<'_>, LocalModelPickDataError> {
    self.picker.read_mesh_view(self.mesh, self.mode)
  }
}

impl ResolvedLocalModelPicker for ResolvedAttributeMeshPicker<'_> {
  fn tolerance(&self) -> Option<IntersectTolerance> {
    self.tolerance
  }

  fn primitive_count(&self) -> Result<u32, LocalModelPickDataError> {
    Ok(self.mesh_view()?.primitive_count() as u32)
  }

  fn ray_query_local(
    &self,
    req: LocalRayQuery,
    v: &mut dyn LocalRayHitVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    let mesh = self.mesh_view()?;
    let config = MeshBufferIntersectConfig {
      tolerance_local: req.local_tolerance,
      triangle_face: FaceSide::Double,
    };
    let hits = mesh.ray_intersect_iter(req.local_ray, &config);
    Ok(visit_mesh_ray_hits(&req.local_ray, hits, |i| i as u32, v))
  }

  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    let mesh = self.mesh_view()?;
    // todo extra_screen_space_tolerance and line point tolerance
    Ok(visit_frustum_primitives(
      mesh.primitive_iter(),
      |p| frustum_test_primitive(p, req.helper, req.local_frustum, req.policy),
      v,
    ))
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
