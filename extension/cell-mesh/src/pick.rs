use rendiation_geometry::*;
use rendiation_mesh_core::{AbstractMesh, AbstractMeshIntersectionExt};
use rendiation_scene_geometry_query::*;

use crate::*;

// sm -> local bbox
pub fn use_cell_mesh_local_bounding(
  cx: &mut impl DBHookCxLike,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>> {
  let local_boxes = cx
    .use_dual_query::<CellMeshUnitsBuffer>()
    .use_dual_query_execute_map(cx, || {
      |_, buffer| {
        let buffer: &[CellMeshUnitData] = cast_slice(&buffer);
        let box3: Box3<f32> = buffer
          .iter()
          // center should be considered
          .flat_map(|v| [v.p1, v.p2, v.p3, v.p4, v.center])
          .collect();
        box3
      }
    });

  let relation = cx.use_db_rev_ref_tri_view::<StandardModelCellMeshPayload>();
  let bbox = local_boxes.fanout(relation, cx);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  bbox.fanout(relation, cx)
}

pub fn use_cell_mesh_picker(cx: &mut impl DBHookCxLike) -> Option<CellMeshPicker> {
  cx.when_resolve_stage(|| CellMeshPicker {
    units: read_global_db_component(),
    shrink_ratio: read_global_db_component(),
    sm_to_std_model: read_global_db_foreign_key(),
    std_model_to_cell_mesh: read_global_db_foreign_key(),
  })
}

pub struct CellMeshPicker {
  pub units: ComponentReadView<CellMeshUnitsBuffer>,
  pub shrink_ratio: ComponentReadView<CellMeshShrinkRatio>,
  pub sm_to_std_model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  pub std_model_to_cell_mesh: ForeignKeyReadView<StandardModelCellMeshPayload>,
}

impl CellMeshPicker {
  fn mesh_view(&self, idx: EntityHandle<SceneModelEntity>) -> Option<CellMeshPickView<'_>> {
    let std_model = self.sm_to_std_model.get(idx)?;
    let cell_mesh = self.std_model_to_cell_mesh.get(std_model)?;
    let units = self.units.get(cell_mesh)?;
    let units = cast_slice(units);
    let shrink_ratio = self.shrink_ratio.get_value(cell_mesh)?;
    Some(CellMeshPickView {
      lines: units,
      shrink_ratio,
    })
  }
}

struct CellMeshPickView<'a> {
  lines: &'a [CellMeshUnitData],
  shrink_ratio: f32,
}

impl<'a> AbstractMesh for CellMeshPickView<'a> {
  type Primitive = Triangle<Vec3<f32>>;
  fn primitive_count(&self) -> usize {
    self.lines.len() * 2
  }

  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let unit_index = primitive_index / 2;
    let unit = self.lines.get(unit_index)?;
    let sr = self.shrink_ratio;
    let tri = if primitive_index % 2 == 0 {
      Triangle::new(
        unit.center.lerp(unit.p1, sr),
        unit.center.lerp(unit.p4, sr),
        unit.center.lerp(unit.p3, sr),
      )
    } else {
      Triangle::new(
        unit.center.lerp(unit.p1, sr),
        unit.center.lerp(unit.p3, sr),
        unit.center.lerp(unit.p2, sr),
      )
    };
    Some(tri)
  }
}

impl LocalModelPicker for CellMeshPicker {
  fn bounding_enlarge_tolerance(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<IntersectTolerance>> {
    let _ = self.mesh_view(idx)?;
    Some(None)
  }

  fn ray_query_local_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    _extra_screen_space_tolerance: f32,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<MeshBufferHitPoint> {
    *self
      .mesh_view(idx)?
      .ray_intersect_nearest(local_ray, &FaceSide::Double)
  }

  fn ray_query_local_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    local_ray: Ray3<f32>,
    _local_tolerance: f32,
    _extra_screen_space_tolerance: f32,
    results: &mut Vec<MeshBufferHitPoint>,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<()> {
    self
      .mesh_view(idx)?
      .ray_intersect_all(local_ray, &FaceSide::Double, results);
    Some(())
  }

  fn frustum_query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    f: &Frustum,
    helper: Option<&FrustumIntersectionTestHelper<f32>>,
    policy: ObjectTestPolicy,
    _extra_screen_space_tolerance: f32,
    _world_mat: &Mat4<f64>,
    _camera_ctx: &CameraQueryCtx,
  ) -> Option<bool> {
    let r = frustum_test_abstract_mesh(&self.mesh_view(idx)?, policy, |t| match policy {
      ObjectTestPolicy::Intersect => frustum_intersect_triangle(helper, f, t.a, t.b, t.c),
      ObjectTestPolicy::Contains => f.contains(&t.a) && f.contains(&t.b) && f.contains(&t.c),
    });

    Some(r)
  }
}
