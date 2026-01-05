use rendiation_geometry::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct WideLineSceneModelWorldBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideLineSceneModelWorldBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let local_boxes = cx
      .use_dual_query::<WideLineMeshBuffer>()
      .use_dual_query_execute_map(cx, || {
        |_, buffer| {
          let buffer: &[WideLineVertex] = cast_slice(&buffer);
          let box3: Box3<f32> = buffer.iter().flat_map(|v| [v.start, v.end]).collect();
          box3
        }
      });

    let relation = cx.use_db_rev_ref_tri_view::<SceneModelWideLineRenderPayload>();
    let sm_line_local_bounding = local_boxes.fanout(relation, cx);

    let scene_model_world_mat = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);

    // todo, materialize
    scene_model_world_mat
      .dual_query_intersect(sm_line_local_bounding)
      .dual_query_map(|(mat, local)| {
        let f64_box = Box3::new(local.min.into_f64(), local.max.into_f64());
        f64_box.apply_matrix_into(mat)
      })
  }
}

pub fn use_wide_line_picker(cx: &mut impl DBHookCxLike) -> Option<WideLinePicker> {
  let wide_line_sm_bounding = cx
    .use_shared_dual_query_view(WideLineSceneModelWorldBounding)
    .use_assure_result(cx);

  cx.when_resolve_stage(|| WideLinePicker {
    lines: read_global_db_component(),
    line_width: read_global_db_component(),
    relation: read_global_db_foreign_key(),
    sm_bounding: wide_line_sm_bounding
      .expect_resolve_stage()
      .mark_entity_type()
      .into_boxed(),
  })
}

pub struct WideLinePicker {
  pub lines: ComponentReadView<WideLineMeshBuffer>,
  pub relation: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  pub sm_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub line_width: ComponentReadView<WideLineWidth>,
}

impl LocalModelPicker for WideLinePicker {
  /// the local tolerance is totally optional(return 0)
  fn compute_local_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    target_world: Mat4<f64>,
  ) -> Option<f32> {
    let line = self.relation.get(idx)?;
    let target_world_center = self.sm_bounding.access(&idx)?.center();
    let line_width = self.line_width.get_value(line)?;
    let pick_line_tolerance = IntersectTolerance::new(line_width / 2., ToleranceType::ScreenSpace);

    ctx
      .compute_local_tolerance(
        pick_line_tolerance,
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
    let line = self.relation.get(idx)?;
    let lines = self.lines.get(line)?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);

    *WideLinePickView { lines }.ray_intersect_nearest(local_ray, &local_tolerance)
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
  ) -> Option<()> {
    let line = self.relation.get(idx)?;
    let lines = self.lines.get(line)?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);

    WideLinePickView { lines }.ray_intersect_all(local_ray, &local_tolerance, results);
    Some(())
  }
}

struct WideLinePickView<'a> {
  lines: &'a [WideLineVertex],
}

impl<'a> AbstractMesh for WideLinePickView<'a> {
  type Primitive = LineSegment<Vec3<f32>>;
  fn primitive_count(&self) -> usize {
    self.lines.len()
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let line = self.lines.get(primitive_index)?;
    Some(LineSegment::new(line.start, line.end))
  }
}
