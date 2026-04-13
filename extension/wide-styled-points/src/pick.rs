use rendiation_geometry::Box3;

use crate::*;

pub struct WideStyledPointsSceneModelLocalBounding;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for WideStyledPointsSceneModelLocalBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f32>>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let local_boxes = cx
      .use_dual_query::<WidesStyledPointsMeshBuffer>()
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
