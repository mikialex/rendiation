use rendiation_geometry::Box3;

// use rendiation_scene_geometry_query::LocalModelPicker;
use crate::*;

pub struct WideStyledPointsSceneModelLocalBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideStyledPointsSceneModelLocalBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let local_boxes = cx
      .use_dual_query::<WideStyledPointsMeshBuffer>()
      .use_dual_query_execute_map(cx, || {
        |_, buffer| {
          let mut bbox = Box3::empty();
          let buffer: &[WideStyledPointVertex] = cast_slice(&buffer);
          for v in buffer {
            bbox.expand_by_point(v.position);
          }
          bbox
        }
      });

    let relation = cx.use_db_rev_ref_tri_view::<SceneModelWideStyledPointsRenderPayload>();
    local_boxes.fanout(relation, cx)
  }
}

// pub fn use_wide_points_picker(cx: &mut impl DBHookCxLike) -> Option<WidePointsPicker> {
//   cx.when_resolve_stage(|| WidePointsPicker {
//     points: read_global_db_component(),
//     relation: read_global_db_foreign_key(),
//   })
// }

// pub struct WidePointsPicker {
//   pub points: ComponentReadView<WideStyledPointsMeshBuffer>,
//   pub relation: ForeignKeyReadView<SceneModelWideStyledPointsRenderPayload>,
// }

// impl LocalModelPicker for WidePointsPicker {
//   fn bounding_enlarge_tolerance(
//     &self,
//     idx: EntityHandle<SceneModelEntity>,
//   ) -> Option<Option<IntersectTolerance>> {
//     let point = self.relation.get(idx)?;
//     let mesh = self.points.get(point)?;
//     // let line_width = self.line_width.get_value(point)?;
//     // let pick_line_tolerance = IntersectTolerance::new(line_width / 2., ToleranceType::ScreenSpace);
//     // Some(Some(pick_line_tolerance))
//     todo!()
//   }

//   fn ray_query_local_nearest(
//     &self,
//     idx: EntityHandle<SceneModelEntity>,
//     local_ray: Ray3<f32>,
//     local_tolerance: f32,
//   ) -> Option<MeshBufferHitPoint> {
//     // let point = self.relation.get(idx)?;
//     // let lines = self.lines.get(point)?;

//     // // here we assume the buffer is correctly aligned
//     // let lines = cast_slice(lines);

//     // *WidePointPickView { lines }.ray_intersect_nearest(local_ray, &local_tolerance)
//     todo!()
//   }

//   fn ray_query_local_all(
//     &self,
//     idx: EntityHandle<SceneModelEntity>,
//     local_ray: Ray3<f32>,
//     _local_tolerance: f32,
//     results: &mut Vec<MeshBufferHitPoint>,
//   ) -> Option<()> {
//     let point = self.relation.get(idx)?;
//     let points = self.points.get(point)?;

//     // here we assume the buffer is correctly aligned
//     let points: &[WideStyledPointVertex] = cast_slice(points);

//     let camera_proj: Mat4<f32> = todo!();
//     let camera_world: Mat4<f32> = todo!();
//     let object_world: Mat4<f32> = todo!();

//     let local_to_ndc: Mat4<f32> = todo!();

//     for p in points {
//       let p_in_ndc = p.position.apply_matrix_into(local_to_ndc);
//       // do test in screen space if we have hit, then go back to world

//       // let tri
//     }

//     // WidePointPickView { lines }.ray_intersect_all(local_ray, &local_tolerance, results);
//     // Some(())
//     todo!()
//   }
// }

// // struct WidePointPickView<'a> {
// //   lines: &'a [WideStyledPointVertex],
// // }

// // impl<'a> AbstractMesh for WidePointPickView<'a> {
// //   type Primitive = Point<Vec3<f32>>;
// //   fn primitive_count(&self) -> usize {
// //     self.lines.len()
// //   }

// //   fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
// //     let point = self.lines.get(primitive_index)?;
// //     Some(LineSegment::new(point.start, point.end))
// //   }
// // }
