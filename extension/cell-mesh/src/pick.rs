use std::{alloc::Allocator, ops::ControlFlow};

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
  fn read_view(
    &self,
    cell_mesh: EntityHandle<CellMeshEntity>,
  ) -> Result<CellMeshPickView<'_>, LocalModelPickDataError> {
    let err = LocalModelPickDataError;
    let units = self
      .units
      .get(cell_mesh)
      .ok_or(err("cell mesh units buffer"))?;
    let shrink_ratio = self
      .shrink_ratio
      .get_value(cell_mesh)
      .ok_or(err("cell mesh shrink ratio"))?;
    Ok(CellMeshPickView {
      lines: cast_slice(units),
      shrink_ratio,
    })
  }
}

impl LocalModelPickerResolver for CellMeshPicker {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    let std_model = self.sm_to_std_model.get(idx)?;
    let cell_mesh = self.std_model_to_cell_mesh.get(std_model)?;
    let view = self.read_view(cell_mesh);
    Some(view.map(|view| Box::new_in(view, alloc) as _))
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
    let tri = if primitive_index.is_multiple_of(2) {
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

impl ResolvedLocalModelPicker for CellMeshPickView<'_> {
  fn tolerance(&self) -> Option<IntersectTolerance> {
    None
  }

  fn primitive_count(&self) -> Result<u32, LocalModelPickDataError> {
    Ok(self.lines.len() as u32)
  }

  fn ray_query_local(
    &self,
    req: LocalRayQuery,
    v: &mut dyn LocalRayHitVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    // todo extra_screen_space_tolerance
    let hits = self.ray_intersect_iter(req.local_ray, &FaceSide::Double);
    Ok(visit_mesh_ray_hits(
      &req.local_ray,
      hits,
      |i| (i / 2) as u32,
      v,
    ))
  }

  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError> {
    // todo extra_screen_space_tolerance
    Ok(visit_frustum_quads(
      self.primitive_iter(),
      req.policy,
      req.helper,
      req.local_frustum,
      v,
    ))
  }
}
