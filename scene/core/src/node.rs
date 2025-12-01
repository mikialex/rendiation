use crate::*;

declare_entity!(SceneNodeEntity);
declare_foreign_key!(SceneNodeParentIdx, SceneNodeEntity, SceneNodeEntity);

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
    .declare_foreign_key::<SceneNodeParentIdx>()
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
    writer.new_entity(|w| {
      w.write::<SceneNodeVisibleComponent>(&self.visible)
        .write::<SceneNodeLocalMatrixComponent>(&self.local_matrix)
        .write::<SceneNodeParentIdx>(&self.parent)
    })
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

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
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

pub fn use_global_node_world_mat(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, Mat4<f64>>> {
  let c = global_node_derive_of::<SceneNodeLocalMatrixComponent, _>(node_world_mat);
  cx.use_shared_dual_query(c)
}

pub fn use_global_node_net_visible(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, bool>> {
  let c = global_node_derive_of::<SceneNodeVisibleComponent, _>(node_net_visible);
  cx.use_shared_dual_query(c)
}

pub fn use_global_node_world_mat_view(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynQuery<RawEntityHandle, Mat4<f64>>> {
  let c = global_node_derive_of::<SceneNodeLocalMatrixComponent, _>(node_world_mat);
  cx.use_shared_dual_query_view(c)
}

pub fn use_global_node_net_visible_view(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynQuery<RawEntityHandle, bool>> {
  let c = global_node_derive_of::<SceneNodeVisibleComponent, _>(node_net_visible);
  cx.use_shared_dual_query_view(c)
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

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let connectivity_rev_view = cx.use_shared_compute(GlobalNodeConnectivity);
    let connectivity_change = use_connectivity_change(cx);
    let connectivity_view = get_db_view::<SceneNodeParentIdx>().filter_map(|v| v);
    let payload_change = cx.use_query_change::<C>();
    let payload_source = get_db_view::<C>();

    let derived = cx.use_shared_hash_map::<RawEntityHandle, C::Data>();

    let f = self.0;

    connectivity_rev_view
      .join(connectivity_change)
      .join(payload_change)
      .map_spawn_stage_in_thread(
        cx,
        |((_, connectivity_change), payload_change)| {
          connectivity_change.has_item_hint() || payload_change.has_item_hint()
        },
        move |((connectivity_rev_view, connectivity_change), payload_change)| {
          let mut d = derived.write();
          let changes = compute_tree_derive(
            &mut d,
            f,
            payload_source,
            payload_change,
            connectivity_view,
            connectivity_rev_view,
            connectivity_change,
          );

          if d.capacity() > d.len() * 2 {
            d.shrink_to_fit();
          }
          drop(d);

          DualQuery {
            view: derived.make_read_holder(),
            delta: Arc::new(changes),
            is_delta_retainable: true,
          }
        },
      )
  }
}
