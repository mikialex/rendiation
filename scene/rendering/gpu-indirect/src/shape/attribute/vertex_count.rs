use rendiation_mesh_core::AttributeSemantic;

use crate::*;

pub fn use_bindless_mesh_vertex_count(
  cx: &mut impl DBHookCxLike,
  index_data_source: AttributeIndexDataSource,
  vertex_data_source: AttributeVertexDataSource,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, u32>> {
  let index_counts = index_data_source
    .map_changes(|v| v.count as u32)
    .use_change_to_dual_query_in_spawn_stage(cx);

  let vertex_counts = vertex_data_source
    .map_changes(|v| v.count as u32)
    .use_change_to_dual_query_in_spawn_stage(cx);

  let all_position_vertex_relations = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferSemantic>()
    .dual_query_filter_map(|v| {
      if let AttributeSemantic::Positions = v {
        Some(())
      } else {
        None
      }
    });

  let (all_position_vertex_relations, all_position_vertex_relations_) =
    all_position_vertex_relations.fork();

  let position_vertex_count = vertex_counts
    .dual_query_filter_by_set(all_position_vertex_relations)
    .dual_query_boxed();

  // todo, impl reduce to simplify this logic
  let relation_ref_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_filter_map(|v| v)
    .dual_query_boxed()
    .dual_query_filter_by_set(all_position_vertex_relations_)
    .dual_query_boxed()
    .use_dual_query_hash_reverse_checked_one_one(cx)
    .dual_query_boxed()
    .use_dual_query_hash_many_to_one(cx);

  let position_vertex_count = position_vertex_count
    .fanout(relation_ref_mesh, cx)
    .dual_query_boxed();

  index_counts
    .dual_query_union(position_vertex_count, |(index, position)| {
      match (index, position) {
        (None, None) => None,
        (None, Some(c)) => Some(c),
        (Some(c), None) => Some(c),
        (Some(c), Some(_)) => Some(c),
      }
    })
    .fanout(
      cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>(),
      cx,
    )
    .fanout(
      cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>(),
      cx,
    )
    .dual_query_boxed()
}
