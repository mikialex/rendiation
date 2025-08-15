use crate::*;

declare_entity!(SceneNodeEntity);
declare_component!(SceneNodeParentIdx, SceneNodeEntity, Option<RawEntityHandle>);

// using f64 float for better precision(at least for computing)
// the underlayer world space position also using f64.
//
// the render precision is based on f32 around camera.
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

fn use_connectivity_change(
  cx: &mut impl DBHookCxLike,
) -> UseResult<impl Query<Key = RawEntityHandle, Value = ValueChange<RawEntityHandle>> + 'static> {
  cx.use_query_change::<SceneNodeParentIdx>()
    .map(|v| v.delta_filter_map(|v| v))
}

pub struct GlobalNodeConnectivity;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for GlobalNodeConnectivity {
  type Result = RevRefContainerRead<RawEntityHandle, RawEntityHandle>;

  fn use_logic(&self, cx: &mut Cx) -> TaskUseResult<Self::Result> {
    let connectivity_change = use_connectivity_change(cx);
    cx.use_rev_ref(connectivity_change)
  }
}

pub fn node_net_visible(this: &bool, parent: Option<&bool>) -> bool {
  parent.map(|p| *p && *this).unwrap_or(*this)
}

pub fn node_world_mat(this: &Mat4<f64>, parent: Option<&Mat4<f64>>) -> Mat4<f64> {
  parent.map(|p| *p * *this).unwrap_or(*this)
}

pub type DeriveDataDualQuery<T> = DualQuery<
  LockReadGuardHolder<FastHashMap<RawEntityHandle, T>>,
  Arc<FastHashMap<RawEntityHandle, ValueChange<T>>>,
>;

#[track_caller]
pub fn use_global_node_world_mat(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, Mat4<f64>>> {
  let c = global_node_derive_of::<SceneNodeLocalMatrixComponent, _>(node_world_mat);
  cx.use_shared_dual_query(c)
}

#[track_caller]
pub fn use_global_node_net_visible(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, bool>> {
  let c = global_node_derive_of::<SceneNodeVisibleComponent, _>(node_net_visible);
  cx.use_shared_dual_query(c)
}

pub struct GlobalNodeDerive<F, C>(pub F, PhantomData<C>);
pub fn global_node_derive_of<C, F>(f: F) -> GlobalNodeDerive<F, C> {
  GlobalNodeDerive(f, PhantomData)
}

impl<C, Cx, F> SharedResultProvider<Cx> for GlobalNodeDerive<F, C>
where
  C: ComponentSemantic,
  Cx: DBHookCxLike,
  F: Fn(&C::Data, Option<&C::Data>) -> C::Data + Send + Sync + 'static + Copy,
{
  type Result = DeriveDataDualQuery<C::Data>;

  fn use_logic(&self, cx: &mut Cx) -> TaskUseResult<Self::Result> {
    let connectivity_rev_view = cx.use_shared_compute(GlobalNodeConnectivity);
    let connectivity_change = use_connectivity_change(cx);
    let connectivity_view = get_db_view::<SceneNodeParentIdx>().filter_map(|v| v);
    let visible_change = cx.use_query_change::<C>();
    let visible_source = get_db_view::<C>();

    let derived = cx.use_shared_hash_map::<RawEntityHandle, C::Data>();

    cx.use_global_shared_future(connectivity_rev_view.into_spawn_stage_future().map(
      |connectivity_rev_view| {
        let visible_change = visible_change.expect_spawn_stage_ready();
        let connectivity_change = connectivity_change.expect_spawn_stage_ready();
        let f = self.0;
        async move {
          let connectivity_rev_view = connectivity_rev_view.await;

          let changes = compute_tree_derive(
            &mut derived.write(),
            f,
            visible_source,
            visible_change,
            connectivity_view,
            connectivity_rev_view,
            connectivity_change,
          );

          DualQuery {
            view: derived.make_read_holder(),
            delta: Arc::new(changes),
          }
        }
      },
    ))
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
