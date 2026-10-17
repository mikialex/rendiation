use std::{alloc::Allocator, ops::ControlFlow};

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
  fn read_view(
    &self,
    line: EntityHandle<WideLineModelEntity>,
  ) -> Result<WideLinePickView<'_>, LocalModelPickDataError> {
    let err = LocalModelPickDataError;
    let lines = self.lines.get(line).ok_or(err("wide line buffer"))?;
    let is_line_strip = self
      .is_line_strip
      .get_value(line)
      .ok_or(err("wide line strip flag"))?;
    let line_width = self
      .line_width
      .get_value(line)
      .ok_or(err("wide line width"))?;

    // here we assume the buffer is correctly aligned
    let lines = cast_slice(lines);
    Ok(WideLinePickView {
      lines,
      is_line_strip,
      line_width,
    })
  }
}

impl LocalModelPickerResolver for WideLinePicker {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    let line = self.relation.get(idx)?;
    let view = self.read_view(line);
    Some(view.map(|view| Box::new_in(view, alloc) as _))
  }
}

struct WideLinePickView<'a> {
  lines: &'a [WideLineVertex],
  is_line_strip: bool,
  line_width: f32,
}

impl ResolvedLocalModelPicker for WideLinePickView<'_> {
  fn tolerance(&self) -> Option<IntersectTolerance> {
    Some(IntersectTolerance::new(
      self.line_width / 2.,
      ToleranceType::ScreenSpace,
    ))
  }

  fn primitive_count(&self) -> Result<u32, LocalModelPickDataError> {
    Ok(AbstractMesh::primitive_count(self) as u32)
  }

  fn ray_query_local(
    &self,
    req: LocalRayQuery,
    v: &mut dyn LocalRayHitVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    let hits = self.ray_intersect_iter(req.local_ray, &req.local_tolerance);
    Ok(visit_mesh_ray_hits(&req.local_ray, hits, |i| i as u32, v))
  }

  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    // todo extra_screen_space_tolerance
    Ok(visit_frustum_primitives(
      self.primitive_iter(),
      |line| frustum_test_line_segment(req.helper, req.local_frustum, line, req.policy),
      v,
    ))
  }
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
