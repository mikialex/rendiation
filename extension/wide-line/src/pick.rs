use rendiation_geometry::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct WideLineSceneModelWorldBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideLineSceneModelWorldBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>;

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
    lines: global_entity_component_of::<WideLineMeshBuffer>().read(),
    line_width: global_entity_component_of::<WideLineWidth>().read(),
    relation: global_entity_component_of::<SceneModelWideLineRenderPayload>().read_foreign_key(),
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
  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<bool> {
    let mesh_world_bounding = self.sm_bounding.access(&idx)?;
    IntersectAble::<_, bool, _>::intersect(&ctx.world_ray, &mesh_world_bounding, &()).into()
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_ray: Ray3<f32>,
    target_world: Mat4<f64>,
  ) -> Option<MeshBufferHitPoint> {
    let line = self.relation.get(idx)?;
    let lines = self.lines.get(line)?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);

    let target_world_center = self.sm_bounding.access(&idx)?.center();

    let line_width = self.line_width.get_value(line)?;
    let pick_line_tolerance = IntersectTolerance::new(line_width / 2., ToleranceType::ScreenSpace);

    let line_tolerance_local =
      ctx.compute_local_tolerance(pick_line_tolerance, target_world, target_world_center);

    *WideLinePickView { lines }.ray_intersect_nearest(local_ray, &line_tolerance_local)
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
