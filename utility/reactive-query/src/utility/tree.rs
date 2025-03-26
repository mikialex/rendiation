use rendiation_abstract_tree::*;

use crate::*;

pub type ReactiveTreeConnectivity<K> = BoxedDynReactiveOneToManyRelation<K, K>;
pub type ReactiveTreePayload<K, T> = BoxedDynReactiveQuery<K, T>;

pub fn tree_payload_derive_by_parent_decide_children<K, T>(
  connectivity: ReactiveTreeConnectivity<K>,
  payload: ReactiveTreePayload<K, T>,
  derive_logic: impl Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
) -> impl ReactiveQuery<Key = K, Value = T>
where
  K: CKey,
  T: CValue,
{
  TreeDerivedData {
    data: Default::default(),
    payload_source: payload,
    connectivity_source: connectivity,
    derive_logic,
  }
}

struct TreeDerivedData<K, T, F> {
  /// where the actually derived data stored
  data: Arc<RwLock<FastHashMap<K, T>>>,
  payload_source: ReactiveTreePayload<K, T>,
  connectivity_source: ReactiveTreeConnectivity<K>,
  derive_logic: F,
}

struct TreeDerivedDataCompute<K, T, F, S, C> {
  data: Arc<RwLock<FastHashMap<K, T>>>,
  derive_logic: F,
  payload_source: S,
  connectivity_source: C,
}

impl<K, T, F, S, C> AsyncQueryCompute for TreeDerivedDataCompute<K, T, F, S, C>
where
  K: CKey,
  T: CValue,
  S: AsyncQueryCompute<Key = K, Value = T>,
  C: AsyncQueryCompute<Key = K, Value = K, View: MultiQuery<Key = K, Value = K>>,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;
  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let derive_logic = self.derive_logic;
    let data = self.data.clone();
    let payload_source = self.payload_source.create_task(cx);
    let connectivity_source = self.connectivity_source.create_task(cx);
    let joined = futures::future::join(payload_source, connectivity_source);
    cx.then_spawn(joined, move |(payload_source, connectivity_source)| {
      TreeDerivedDataCompute {
        data: data.clone(),
        derive_logic,
        payload_source,
        connectivity_source,
      }
      .resolve()
    })
  }
}

impl<K, T, F, S, C> QueryCompute for TreeDerivedDataCompute<K, T, F, S, C>
where
  K: CKey,
  T: CValue,
  S: QueryCompute<Key = K, Value = T>,
  C: QueryCompute<Key = K, Value = K, View: MultiQuery<Key = K, Value = K>>,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  type Key = K;
  type Value = T;
  type Changes = Arc<FastHashMap<K, ValueChange<T>>>;
  type View = LockReadGuardHolder<FastHashMap<K, T>>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (payload_change, current_source) = self.payload_source.resolve();
    let (connectivity_change, current_connectivity) = self.connectivity_source.resolve();
    let current_inv_connectivity = current_connectivity.clone();

    // step1: find the update root
    // we have a change set as input. change set is the composition of the connectivity change
    // and the payload change. for each item in change set, we recursively check it parent if any
    // of the parent exist in the change set, if not, we have a update root

    let mut update_roots = FastHashSet::default();

    let payload_change_range = payload_change.iter_key_value().map(|(k, _)| k);

    let connectivity_change_range = connectivity_change
      .iter_key_value()
      .map(|(k, _)| k)
      .filter(|k| payload_change.access(k).is_none()); // remove the payload_change_range part

    let is_in_change_set = |k| payload_change.contains(&k) || connectivity_change.contains(&k);

    // if we have a very branchy tree, and only root and leaves contains change, it's hard to
    // find the update root is the tree root because early return is not effective. To solve this
    // issue, we may continuously populate the change set by traversing the changeset item's sub
    // tree into change set but it's not a good solution, because it's hard to parallelize  and
    // the most important is that we can assume our tree is not that deep
    for change in payload_change_range.chain(connectivity_change_range) {
      let mut current_check = change.clone();
      loop {
        if let Some(parent) = current_connectivity.access(&current_check) {
          if is_in_change_set(parent.clone()) {
            break;
          }
          current_check = parent.clone();
        } else {
          update_roots.insert(change.clone());
          break;
        }
      }
    }

    // step2: do derive update from all update roots
    // maybe could using some forms of dynamic parallelism
    let mut derive_tree = self.data.write();
    let mut derive_changes = FastHashMap::default();
    let collector = QueryMutationCollectorPtr {
      delta: &mut derive_changes as *mut _,
      target: (&mut derive_tree as &mut FastHashMap<K, T>) as *mut _,
    };

    let ctx = Ctx {
      derive: collector,
      source: &current_source,
      connectivity: &current_inv_connectivity,
      parent_connectivity: &current_connectivity,
      derive_logic: self.derive_logic,
    };
    for root in update_roots {
      let mut root = TreeMutNode {
        phantom: PhantomData,
        idx: root,
        ctx: &ctx,
      };
      root.traverse_pair_subtree_mut(&mut |node, parent| {
        if node.update(parent.as_deref()) {
          NextTraverseVisit::VisitChildren
        } else {
          NextTraverseVisit::SkipChildren
        }
      })
    }

    drop(derive_tree);
    let d = Arc::new(derive_changes);
    let v = self.data.make_read_holder();
    (d, v)
  }
}

impl<K, T, F> ReactiveQuery for TreeDerivedData<K, T, F>
where
  K: CKey,
  T: CValue,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  type Key = K;
  type Value = T;
  type Compute = impl QueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    TreeDerivedDataCompute {
      data: self.data.clone(),
      derive_logic: self.derive_logic,
      payload_source: self.payload_source.describe(cx),
      connectivity_source: self.connectivity_source.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.data.write().shrink_to_fit();
    self.payload_source.request(request);
    self.connectivity_source.request(request);
  }
}

struct QueryMutationCollectorPtr<K, T> {
  delta: *mut FastHashMap<K, ValueChange<T>>,
  target: *mut FastHashMap<K, T>,
}

impl<K: CKey, T: CValue> QueryMutationCollectorPtr<K, T> {
  pub fn get_derive(&self, key: K) -> Option<&T> {
    unsafe { (*self.target).get(&key) }
  }
  pub fn as_mutable(&self) -> impl QueryLikeMutateTarget<K, T> {
    unsafe {
      QueryMutationCollector {
        delta: (&mut *self.delta),
        target: (&mut *self.target),
      }
    }
  }
}

impl<K, T> Clone for QueryMutationCollectorPtr<K, T> {
  fn clone(&self) -> Self {
    Self {
      delta: self.delta,
      target: self.target,
    }
  }
}

#[derive(Clone)]
struct TreeMutNode<'a, K, T, F> {
  phantom: PhantomData<T>,
  idx: K,
  ctx: &'a Ctx<'a, K, T, F>,
}

struct Ctx<'a, K, T, F> {
  derive: QueryMutationCollectorPtr<K, T>,
  source: &'a dyn DynQuery<Key = K, Value = T>,
  connectivity: &'a dyn DynMultiQuery<Key = K, Value = K>,
  parent_connectivity: &'a dyn DynQuery<Key = K, Value = K>,
  derive_logic: F,
}

impl<K, T, F> TreeMutNode<'_, K, T, F>
where
  K: CKey,
  T: CValue,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  pub fn get_derive(&self) -> &T {
    self.ctx.derive.get_derive(self.idx.clone()).unwrap()
  }
  /// return has actually changed
  pub fn set_derive(&self, d: T) -> bool {
    let p = self
      .ctx
      .derive
      .as_mutable()
      .set_value(self.idx.clone(), d.clone());
    if let Some(p) = p {
      p != d
    } else {
      true
    }
  }
  /// return has actually changed
  pub fn update(&mut self, parent: Option<&Self>) -> bool {
    let parent_derive = parent.map(|parent| parent.get_derive());
    let self_source = self.ctx.source.access_dyn(&self.idx).unwrap();
    self.set_derive((self.ctx.derive_logic)(&self_source, parent_derive))
  }
}

impl<K: CKey, T, F> AbstractTreeMutNode for TreeMutNode<'_, K, T, F> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    if let Some(children) = self.ctx.connectivity.access_multi(&self.idx) {
      for idx in children {
        visitor(&mut TreeMutNode {
          phantom: PhantomData,
          idx,
          ctx: self.ctx,
        });
      }
    }
  }
}

impl<K: CKey, T, F> AbstractParentAddressableMutTreeNode for TreeMutNode<'_, K, T, F> {
  fn get_parent_mut(&mut self) -> Option<Self> {
    self
      .ctx
      .parent_connectivity
      .access_dyn(&self.idx)
      .map(|idx| TreeMutNode {
        phantom: PhantomData,
        idx,
        ctx: self.ctx,
      })
  }
}
