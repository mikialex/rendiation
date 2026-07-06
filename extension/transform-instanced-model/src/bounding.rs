use crate::*;

pub fn use_instanced_model_local_bounding(
  cx: &mut impl DBHookCxLike,
  source_bounding: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>>,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>> {
  let all_instance_models_ref_all_source_models =
    cx.use_db_rev_ref_tri_view::<TransformInstancedModelRefSceneModel>();

  let source_bounding = source_bounding
    .fanout(all_instance_models_ref_all_source_models, cx)
    .dual_query_boxed();

  let instance_bounding = source_bounding
    .dual_query_zip(cx.use_dual_query::<TransformInstancedModelInstanceBuffer>())
    .dual_query_boxed()
    .dual_query_zip(cx.use_dual_query::<TransformInstancedModelPerUnitTransform>())
    .dual_query_boxed()
    .dual_query_map(
      |((source_local_bbox, transform_buffer), per_unit_transform)| {
        let source_local_bbox = if let Some(per_unit_transform) = per_unit_transform {
          source_local_bbox.apply_matrix_into(per_unit_transform)
        } else {
          source_local_bbox
        };
        let bbox: Box3<f32> = transform_buffer
          .iter()
          .map(|m| source_local_bbox.apply_matrix_into(*m))
          .collect();
        bbox
      },
    );

  let sm_ref_instance_model =
    cx.use_db_rev_ref_tri_view::<SceneModelTransformInstancedModelPayload>();

  instance_bounding
    .fanout(sm_ref_instance_model, cx)
    .dual_query_boxed()
}
