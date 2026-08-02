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
          let box3: Box3<f32> = buffer.iter().map(|v| v.position).collect();
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
    is_line_strip: read_global_db_component(),
  })
}

pub struct WideLinePicker {
  pub lines: ComponentReadView<WideLineMeshBuffer>,
  pub relation: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  pub line_width: ComponentReadView<WideLineWidth>,
  pub is_line_strip: ComponentReadView<WideLineIsLineStrip>,
}

impl WideLinePicker {
  fn mesh_view(&self, idx: EntityHandle<SceneModelEntity>) -> Option<WideLinePickView<'_>> {
    let line = self.relation.get(idx)?;
    let lines = self.lines.get(line)?;
    let is_line_strip = self.is_line_strip.get_value(line)?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);
    Some(WideLinePickView {
      lines,
      is_line_strip,
    })
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

  fn ray_query_local_nearest(&self, request: LocalRayQueryRequest) -> Option<MeshBufferHitPoint> {
    let LocalRayQueryRequest {
      idx,
      local_ray,
      local_tolerance,
      ..
    } = request;
    // todo extra_screen_space_tolerance
    *self
      .mesh_view(idx)?
      .ray_intersect_nearest(local_ray, &local_tolerance)
  }

  fn ray_query_local_all(&self, request: LocalRayAllQueryRequest) -> Option<()> {
    let LocalRayAllQueryRequest {
      internal:
        LocalRayQueryRequest {
          idx,
          local_ray,
          local_tolerance,
          ..
        },
      results,
    } = request;
    // todo extra_screen_space_tolerance
    self
      .mesh_view(idx)?
      .ray_intersect_all(local_ray, &local_tolerance, results);
    Some(())
  }

  fn frustum_query_local(&self, request: LocalFrustumQueryRequest) -> Option<bool> {
    let LocalFrustumQueryRequest {
      idx,
      local_frustum: f,
      helper,
      policy,
      ..
    } = request;
    // todo extra_screen_space_tolerance
    let r = frustum_test_abstract_mesh(&self.mesh_view(idx)?, policy, |line| {
      frustum_test_line(line, policy, f, helper)
    });

    Some(r)
  }

  fn frustum_query_local_sub_primitives(
    &self,
    request: LocalFrustumSubPrimitiveQueryRequest,
  ) -> Option<()> {
    let LocalFrustumSubPrimitiveQueryRequest {
      internal:
        LocalFrustumQueryRequest {
          idx,
          local_frustum: frustum,
          helper,
          policy,
          ..
        },
      results,
    } = request;
    let view = self.mesh_view(idx)?;

    for (i, line) in view.primitive_iter().enumerate() {
      if frustum_test_line(line, policy, frustum, helper) {
        results.push(i as u32);
      }
    }

    Some(())
  }
}

fn frustum_test_line(
  line: LineSegment3D,
  policy: ObjectTestPolicy,
  f: &Frustum,
  helper: Option<&FrustumIntersectionTestHelper<f32>>,
) -> bool {
  match policy {
    ObjectTestPolicy::Intersect => frustum_intersect_line_segment(helper, f, line.start, line.end),
    ObjectTestPolicy::Contains => f.contains(&line.start) && f.contains(&line.end),
  }
}

struct WideLinePickView<'a> {
  lines: &'a [WideLineVertex],
  is_line_strip: bool,
}

impl<'a> AbstractMesh for WideLinePickView<'a> {
  type Primitive = LineSegment<Vec3<f32>>;
  fn primitive_count(&self) -> usize {
    if self.is_line_strip {
      self.lines.len().saturating_sub(1)
    } else {
      self.lines.len() / 2
    }
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let start_index = if self.is_line_strip {
      primitive_index
    } else {
      primitive_index * 2
    };
    let start = self.lines.get(start_index)?;
    let end = self.lines.get(start_index + 1)?;
    Some(LineSegment::new(start.position, end.position))
  }
}
