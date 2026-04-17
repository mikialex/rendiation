use rendiation_geometry::*;
use rendiation_scene_geometry_query::*;

use crate::*;

pub struct WideLineSceneModelLocalBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideLineSceneModelLocalBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>;
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
    local_boxes.fanout(relation, cx)
  }
}

pub fn use_wide_line_picker(cx: &mut impl DBHookCxLike) -> Option<WideLinePicker> {
  cx.when_resolve_stage(|| WideLinePicker {
    lines: read_global_db_component(),
    line_width: read_global_db_component(),
    relation: read_global_db_foreign_key(),
  })
}

pub struct WideLinePicker {
  pub lines: ComponentReadView<WideLineMeshBuffer>,
  pub relation: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  pub line_width: ComponentReadView<WideLineWidth>,
}

impl WideLinePicker {
  fn mesh_view(&self, idx: EntityHandle<SceneModelEntity>) -> Option<WideLinePickView<'_>> {
    let line = self.relation.get(idx)?;
    let lines = self.lines.get(line)?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);
    Some(WideLinePickView { lines })
  }
}

impl LocalModelPicker for WideLinePicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    let line = self.relation.get(idx)?;
    let line_width = self.line_width.get_value(line)?;
    let pick_line_tolerance = IntersectTolerance::new(line_width / 2., ToleranceType::ScreenSpace);
    Some(Some(pick_line_tolerance))
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
  ) -> Option<MeshBufferHitPoint> {
    *self
      .mesh_view(idx)?
      .ray_intersect_nearest(local_ray, &local_tolerance)
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    local_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
  ) -> Option<()> {
    self
      .mesh_view(idx)?
      .ray_intersect_all(local_ray, &local_tolerance, results);
    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    f: &Frustum,
    policy: ObjectTestPolicy,
  ) -> Option<bool> {
    let r = frustum_test_abstract_mesh(&self.mesh_view(idx)?, policy, |line| match policy {
      ObjectTestPolicy::Intersect => f.contains(&line.start) || f.contains(&line.end),
      ObjectTestPolicy::Contains => f.contains(&line.start) && f.contains(&line.end),
    });

    Some(r)
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
