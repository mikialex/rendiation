use crate::*;

declare_entity!(SceneNodeEntity);
declare_component!(SceneNodeParentIdx, SceneNodeEntity, Option<RawEntityHandle>);

// using f64 float for better precision(at least for computing)
//
// currently the render precision is still based on f32 around origin (0, 0, 0).
// in the future, the render precision will based on f32 around camera, and the position precision will be preserved.
declare_component!(
  SceneNodeLocalMatrixComponent,
  SceneNodeEntity,
  Mat4<f64>,
  Mat4::identity()
);
declare_component!(SceneNodeVisibleComponent, SceneNodeEntity, bool, true);
pub fn register_scene_node_data_model() {
  global_database()
    .declare_entity::<SceneNodeEntity>()
    .declare_component::<SceneNodeParentIdx>()
    .declare_component::<SceneNodeLocalMatrixComponent>()
    .declare_component::<SceneNodeVisibleComponent>();
}

pub struct SceneNodeDataView {
  pub visible: bool,
  pub local_matrix: Mat4<f64>,
  pub parent: Option<RawEntityHandle>,
}

impl SceneNodeDataView {
  pub fn write(self, writer: &mut EntityWriter<SceneNodeEntity>) -> EntityHandle<SceneNodeEntity> {
    writer
      .component_value_writer::<SceneNodeVisibleComponent>(self.visible)
      .component_value_writer::<SceneNodeLocalMatrixComponent>(self.local_matrix)
      .component_value_writer::<SceneNodeParentIdx>(self.parent)
      .new_entity()
  }
}

#[global_registered_query_and_many_one_hash_relation]
pub fn scene_node_connectivity(
) -> impl ReactiveQuery<Key = EntityHandle<SceneNodeEntity>, Value = EntityHandle<SceneNodeEntity>>
{
  global_watch()
    .watch::<SceneNodeParentIdx>()
    .collective_filter_map(|v| v.map(|v| unsafe { EntityHandle::from_raw(v) }))
}

#[global_registered_query]
pub fn scene_node_derive_visible(
) -> impl ReactiveQuery<Key = EntityHandle<SceneNodeEntity>, Value = bool> {
  tree_payload_derive_by_parent_decide_children(
    Box::new(scene_node_connectivity_many_one_relation()),
    global_watch()
      .watch::<SceneNodeVisibleComponent>()
      .into_boxed(),
    |this, parent| parent.map(|p| *p && *this).unwrap_or(*this),
  )
}

#[global_registered_query]
pub fn scene_node_derive_world_mat(
) -> impl ReactiveQuery<Key = EntityHandle<SceneNodeEntity>, Value = Mat4<f64>> {
  tree_payload_derive_by_parent_decide_children(
    Box::new(scene_node_connectivity_many_one_relation()),
    global_watch()
      .watch::<SceneNodeLocalMatrixComponent>()
      .into_boxed(),
    |this, parent| parent.map(|p| *p * *this).unwrap_or(*this),
  )
}
