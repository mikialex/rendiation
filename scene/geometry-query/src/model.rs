use crate::*;

pub trait LocalModelPicker {
  fn compute_local_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    target_world: Mat4<f64>,
  ) -> Option<f32>;

  /// return if intersect with bounding
  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_tolerance: f32,
  ) -> Option<bool>;

  /// should return hit result in local space
  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
  ) -> Option<MeshBufferHitPoint>;

  /// should return hit result in local space
  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
  ) -> Option<()>;
}

impl LocalModelPicker for Vec<Box<dyn LocalModelPicker>> {
  fn compute_local_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    target_world: Mat4<f64>,
  ) -> Option<f32> {
    for provider in self {
      if let Some(hit) = provider.compute_local_tolerance(idx, ctx, target_world) {
        return Some(hit);
      }
    }
    None
  }

  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_tolerance: f32,
  ) -> Option<bool> {
    for provider in self {
      if let Some(hit) = provider.bounding_pre_test(idx, ctx, local_tolerance) {
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
  ) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) = provider.ray_query_local_nearest(idx, local_ray, local_tolerance) {
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
  ) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_local_all(idx, local_ray, local_tolerance, results)
        .is_some()
      {
        return Some(());
      }
    }
    None
  }
}

pub fn use_attribute_mesh_picker(cx: &mut impl DBHookCxLike) -> Option<AttributeMeshPicker> {
  let mesh_vertex_refs = cx
    .use_db_rev_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .use_assure_result(cx);

  let sm_bounding = cx
    .use_shared_dual_query_view(SceneModelByAttributesMeshStdModelWorldBounding)
    .use_assure_result(cx);

  cx.when_resolve_stage(|| AttributeMeshPicker {
    sm_bounding: sm_bounding
      .expect_resolve_stage()
      .mark_entity_type()
      .into_boxed(),
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
  pub sm_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
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
  fn ray_query_local_read_view(
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
  fn compute_local_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    target_world: Mat4<f64>,
  ) -> Option<f32> {
    let target_world_center = self.sm_bounding.access(&idx)?.center();
    ctx
      .compute_local_tolerance(
        self.pick_line_tolerance,
        target_world,
        ctx.camera_world,
        target_world_center,
      )
      .into()
  }

  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_tolerance: f32,
  ) -> Option<bool> {
    let mesh_world_bounding = self.sm_bounding.access(&idx)?;
    let mesh_world_bounding = mesh_world_bounding.enlarge(local_tolerance as f64);
    IntersectAble::<_, bool, _>::intersect(&ctx.world_ray, &mesh_world_bounding, &()).into()
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
  ) -> Option<MeshBufferHitPoint> {
    let config = MeshBufferIntersectConfig {
      tolerance_local: local_tolerance,
      triangle_face: FaceSide::Double,
    };

    *self
      .ray_query_local_read_view(idx)?
      .ray_intersect_nearest(local_ray, &config)
  }

  /// should return hit result in local space
  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
  ) -> Option<()> {
    let config = MeshBufferIntersectConfig {
      tolerance_local: local_tolerance,
      triangle_face: FaceSide::Double,
    };
    self
      .ray_query_local_read_view(idx)?
      .ray_intersect_all(local_ray, &config, results);

    Some(())
  }
}
