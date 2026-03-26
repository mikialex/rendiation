// use crate::*;

// pub struct Text3dSceneModelWorldBounding;

// impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for Text3dSceneModelWorldBounding {
//   type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>;
//   share_provider_hash_type_id! {}

//   fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
//     let local_boxes = cx
//       .use_dual_query::<WidesStyledPointsMeshBuffer>()
//       .use_dual_query_execute_map(cx, || {
//         |_, buffer| {
//           let mut bbox = Box3::empty();
//           let buffer: &[WideStyledPointVertex] = cast_slice(&buffer);
//           for v in buffer {
//             bbox.expand_by_point(v.position);
//           }
//           bbox
//         }
//       });

//     let relation = cx.use_db_rev_ref_tri_view::<SceneModelWideStyledPointsRenderPayload>();
//     let sm_line_local_bounding = local_boxes.fanout(relation, cx);

//     let scene_model_world_mat = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);

//     // todo, materialize
//     scene_model_world_mat
//       .dual_query_intersect(sm_line_local_bounding)
//       .dual_query_map(|(mat, local)| {
//         let f64_box = Box3::new(local.min.into_f64(), local.max.into_f64());
//         f64_box.apply_matrix_into(mat)
//       })
//   }
// }
