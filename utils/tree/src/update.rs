use futures::{Stream, StreamExt};
use incremental::{DeltaPair, ReversibleIncremental};
use reactive::{SignalStreamExt, StreamForker};
use reactive_incremental::CollectionDelta;

use crate::*;

pub trait HierarchyDerivedBase: Clone {
  type Source: IncrementalBase;
  fn build_default(self_source: &Self::Source) -> Self;
}

/// The default value is the none parent case
///
/// We not impose IncrementalHierarchyDerived extends HierarchyDerived
/// because of simplicity.
pub trait IncrementalHierarchyDerived: ReversibleIncremental + HierarchyDerivedBase {
  type DirtyMark: HierarchyDirtyMark;

  /// for any delta of source, check if it will have hierarchy effect.
  /// The dirtyMark will propagate upward and downward in tree
  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::DirtyMark>;

  fn hierarchy_update(
    &mut self,
    self_source: &Self::Source,
    parent_derived: Option<&Self>,
    dirty: &Self::DirtyMark,
    collect: impl FnMut(&mut Self, Self::Delta),
  );
}

#[derive(Default)]
pub struct ParentTreeDirty<M> {
  /// all sub tree change or together includes self
  /// this tag is for skip tree branch updating
  sub_tree_dirty_mark_any: M,

  /// all sub tree change and together includes self.
  /// this tag is for skipping tree dirty marking
  sub_tree_dirty_mark_all: M,
}

pub trait IncrementalChildrenHierarchyDerived:
  ReversibleIncremental + HierarchyDerivedBase
{
  type DirtyMark: HierarchyDirtyMark;

  /// for any delta of source, check if it will have hierarchy effect
  fn filter_children_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::DirtyMark>;

  fn hierarchy_children_update(
    &mut self,
    self_source: &Self::Source,
    children_visitor: impl FnMut(&dyn Fn(&Self)),
    dirty: &Self::DirtyMark,
    collect: impl FnMut(&mut Self, Self::Delta),
  );
}

/// Default should use None state
pub trait HierarchyDirtyMark: PartialEq + Default + 'static {
  fn contains(&self, mark: &Self) -> bool;
  fn intersects(&self, mark: &Self) -> bool;
  fn insert(&mut self, mark: &Self);
  fn all_dirty() -> Self;
}

#[derive(Default)]
pub struct DerivedData<T, M> {
  pub data: T,

  dirty: M,
}

pub struct TreeHierarchyDerivedSystem<T: ReversibleIncremental, Dirty> {
  pub derived_tree: Arc<RwLock<TreeCollection<DerivedData<T, Dirty>>>>,
  // we use boxed here to avoid another generic for tree delta input stream
  pub derived_stream: StreamForker<
    Box<dyn Stream<Item = Vec<CollectionDelta<usize, T::Delta>>> + Unpin + Send + Sync>,
  >,
}

impl<T: ReversibleIncremental, M> Clone for TreeHierarchyDerivedSystem<T, M> {
  fn clone(&self) -> Self {
    Self {
      derived_tree: self.derived_tree.clone(),
      derived_stream: self.derived_stream.clone(),
    }
  }
}

impl<T: ReversibleIncremental, Dirty> TreeHierarchyDerivedSystem<T, Dirty> {
  pub fn new<B, TREE, S, M>(
    tree_delta: impl Stream<Item = Vec<TreeMutation<S>>> + Send + Sync + 'static,
    source_tree: &Arc<TREE>,
  ) -> TreeHierarchyDerivedSystem<T, B::Dirty>
  where
    B: TreeIncrementalDeriveBehavior<T, S, M, TREE::Core, Dirty = Dirty>,
    S: IncrementalBase,
    T: HierarchyDerivedBase<Source = S>,
    Dirty: Default + Send + Sync + 'static,
    M: HierarchyDirtyMark,
    TREE: ShareCoreTree + Send + Sync + 'static,
  {
    let derived_tree = Arc::new(RwLock::new(
      TreeCollection::<DerivedData<T, B::Dirty>>::default(),
    ));

    let derived_tree_c = derived_tree.clone();
    let source_tree = Arc::downgrade(source_tree);

    enum MarkingResult<T, Dirty> {
      UpdateRoot(TreeNodeHandle<DerivedData<T, Dirty>>),
      Remove(usize, T),
      Create(usize, T),
    }

    let derived_stream = tree_delta
      .filter_map_sync(move |deltas| source_tree.upgrade().map(|tree| (tree, deltas)))
      // mark stage
      // do dirty marking, return if should trigger hierarchy change, and the update root
      .map(move |(source_tree, deltas)| {
        let mut marking_results = Vec::with_capacity(deltas.len()); // should be upper bound
        let mut derived_deltas = Vec::with_capacity(deltas.len()); // just estimation
        let mut derived_tree = derived_tree_c.write().unwrap();

        // mark stage
        // do dirty marking, return if should trigger hierarchy change, and the update root
        for delta in deltas {
          let marking = match delta {
            // simply create the default derived. insert into derived tree.
            // we don't care the returned handle, as we assume they are allocated in the same
            // position in the original tree.
            TreeMutation::Create { data, .. } => {
              let data = T::build_default(&data);
              let node = derived_tree.create_node(DerivedData {
                data: data.clone(),
                dirty: Default::default(),
              });
              marking_results.push(MarkingResult::Create(node.index(), data));
              B::marking_dirty(&mut derived_tree, node, M::all_dirty())
            }
            // do pair remove in derived tree
            TreeMutation::Delete(handle) => {
              let handle = derived_tree.recreate_handle(handle);
              let removed = derived_tree.delete_node(handle).unwrap();
              marking_results.push(MarkingResult::Remove(handle.index(), removed.data));
              continue;
            }
            // check if have any hierarchy effect, and do marking
            TreeMutation::Mutate { node, delta } => {
              let handle = derived_tree.recreate_handle(node);
              B::filter_hierarchy_change(&delta)
                .and_then(|dirty_mark| B::marking_dirty(&mut derived_tree, handle, dirty_mark))
            }
            // like update, and we will emit full dirty change
            TreeMutation::Attach {
              parent_target,
              node,
            } => {
              let parent_target = derived_tree.recreate_handle(parent_target);
              let node = derived_tree.recreate_handle(node);
              derived_tree.node_add_child_by(parent_target, node).ok();
              B::marking_dirty(&mut derived_tree, node, M::all_dirty())
            }
            // ditto
            TreeMutation::Detach { node } => {
              let node = derived_tree.recreate_handle(node);
              derived_tree.node_detach_parent(node).ok();
              B::marking_dirty(&mut derived_tree, node, M::all_dirty())
            }
          };
          if let Some(marking) = marking {
            marking_results.push(MarkingResult::UpdateRoot(marking));
          }
        }

        // update stage
        for marking_result in marking_results {
          match marking_result {
            MarkingResult::UpdateRoot(update_root) => {
              // do full tree traverse check, emit all real update as stream
              let tree: &TREE = &source_tree;

              // node maybe deleted
              let node_has_deleted = !derived_tree.is_handle_valid(update_root);

              if !node_has_deleted {
                // if the previous emitted update root is attached to another tree, the new attached
                // tree contains new root that should override the current update root.
                let update_root_is_not_valid = derived_tree.get_node(update_root).parent.is_some();

                if !update_root_is_not_valid {
                  // note, the source tree maybe mutate by other thread in parallel
                  // we must hold a entire tree lock when visit origin tree.
                  // because try recreate handle maybe failed, and even it succeeded, we can not
                  // sure if the recreated handle is valid when get the data we
                  // still could get corrupt data. so we have to use a large lock
                  // here
                  tree.visit_core_tree(|tree| {
                    B::update_derived(tree, &mut derived_tree, update_root, &mut |(idx, pair)| {
                      derived_deltas.push(CollectionDelta::Delta(
                        idx,
                        pair.forward,
                        Some(pair.inverse),
                      ));
                    });
                  });
                }
              }
            }
            MarkingResult::Remove(idx, removed) => {
              removed.expand(|d| {
                derived_deltas.push(CollectionDelta::Remove(idx, d));
              });
            }
            MarkingResult::Create(idx, created) => {
              // we can not use the derived tree data because the previous delta is buffered, and
              // the node at given index maybe removed by later message
              created.expand(|d| {
                derived_deltas.push(CollectionDelta::Delta(idx, d, None));
              });
            }
          }
        }

        // todo, we should do some post processing to ensure the deltas coherency
        derived_deltas
      });

    let boxed: Box<dyn Stream<Item = Vec<CollectionDelta<usize, T::Delta>>> + Unpin + Send + Sync> =
      Box::new(Box::pin(derived_stream));

    Self {
      derived_tree,
      derived_stream: boxed.create_broad_caster(),
    }
  }

  pub fn visit_derived_tree<R>(
    &self,
    v: impl FnOnce(&TreeCollection<DerivedData<T, Dirty>>) -> R,
  ) -> R {
    v(&self.derived_tree.read().unwrap())
  }
}

pub trait TreeIncrementalDeriveBehavior<T: ReversibleIncremental, S: IncrementalBase, M, TREE> {
  type Dirty: Default;
  fn filter_hierarchy_change(change: &S::Delta) -> Option<M>;

  fn marking_dirty(
    tree: &mut TreeCollection<DerivedData<T, Self::Dirty>>,
    change_node: TreeNodeHandle<DerivedData<T, Self::Dirty>>,
    dirty_mark: M,
  ) -> Option<TreeNodeHandle<DerivedData<T, Self::Dirty>>>;

  fn update_derived(
    source_tree: &TREE,
    derived_tree: &mut TreeCollection<DerivedData<T, Self::Dirty>>,
    update_root: TreeNodeHandle<DerivedData<T, Self::Dirty>>,
    derived_delta_sender: &mut impl FnMut((usize, DeltaPair<T>)),
  );
}

pub struct ParentTree;

impl<T: ReversibleIncremental, M, TREE> TreeIncrementalDeriveBehavior<T, T::Source, M, TREE>
  for ParentTree
where
  T: IncrementalHierarchyDerived<DirtyMark = M>,
  M: HierarchyDirtyMark,
  TREE: CoreTree<Node = T::Source>,
{
  type Dirty = ParentTreeDirty<M>;

  // for children, do traverse mark dirty all-mark, skip if all-mark contains new dirty
  // for parent chain, do parent chain traverse mark dirty any-mark,
  // return the root node handle as the update root
  // if the parent chain has been any dirty marked, we return None to skip the update process
  fn marking_dirty(
    tree: &mut TreeCollection<DerivedData<T, Self::Dirty>>,
    change_node: TreeNodeHandle<DerivedData<T, Self::Dirty>>,
    dirty_mark: M,
  ) -> Option<TreeNodeHandle<DerivedData<T, Self::Dirty>>> {
    tree
      .create_node_mut_ptr(change_node)
      .traverse_iter_mut(|node| {
        let child = unsafe { &(*node.node) };
        if child
          .data
          .dirty
          .sub_tree_dirty_mark_all
          .contains(&dirty_mark)
        {
          NextTraverseVisit::SkipChildren
        } else {
          NextTraverseVisit::VisitChildren
        }
      })
      .for_each(|node| {
        let child = unsafe { &mut (*node.node) };
        child.data.dirty.sub_tree_dirty_mark_all.insert(&dirty_mark);
        child.data.dirty.sub_tree_dirty_mark_any.insert(&dirty_mark);
      });

    let mut update_parent = None;
    tree
      .create_node_mut_ptr(change_node)
      .traverse_parent_mut(|n| {
        let n = unsafe { &mut (*n.node) };
        if n.parent.is_none() {
          update_parent = n.handle().into();
        }
        // don't check self, or we will failed to mark sub tree change
        if n.handle() == change_node {
          return true;
        }
        let contains = n.data.dirty.sub_tree_dirty_mark_any.contains(&dirty_mark);
        n.data.dirty.sub_tree_dirty_mark_any.insert(&dirty_mark);
        !contains
      });

    update_parent
  }

  fn update_derived(
    source_tree: &TREE,
    derived_tree: &mut TreeCollection<DerivedData<T, Self::Dirty>>,
    update_root: TreeNodeHandle<DerivedData<T, Self::Dirty>>,
    derived_delta_sender: &mut impl FnMut((usize, DeltaPair<T>)),
  ) {
    derived_tree.traverse_mut_pair(update_root, |node, parent| {
      let node_index = node.handle().index();
      let derived = node.data_mut();

      if derived.dirty.sub_tree_dirty_mark_any == T::DirtyMark::default() {
        NextTraverseVisit::SkipChildren
      } else {
        let parent = parent.map(|parent| &parent.data().data);

        // the source tree maybe out of sync if the other thread mutate the source tree in parallel
        if let Some(source_node) = source_tree.try_recreate_handle(node_index) {
          derived.data.hierarchy_update(
            source_tree.get_node_data(source_node),
            parent,
            &derived.dirty.sub_tree_dirty_mark_all,
            |derive, delta| {
              derived_delta_sender((node_index, derive.make_reverse_delta_pair(delta)))
            },
          );
        }

        derived.dirty.sub_tree_dirty_mark_any = T::DirtyMark::default();
        derived.dirty.sub_tree_dirty_mark_all = T::DirtyMark::default();

        NextTraverseVisit::VisitChildren
      }
    })
  }

  fn filter_hierarchy_change(change: &<T::Source as IncrementalBase>::Delta) -> Option<M> {
    T::filter_hierarchy_change(change)
  }
}

pub struct ChildrenTree;

impl<T: ReversibleIncremental, M, TREE> TreeIncrementalDeriveBehavior<T, T::Source, M, TREE>
  for ChildrenTree
where
  T: IncrementalChildrenHierarchyDerived<DirtyMark = M>,
  M: HierarchyDirtyMark,
  TREE: CoreTree<Node = T::Source>,
{
  type Dirty = M;

  fn filter_hierarchy_change(change: &<T::Source as IncrementalBase>::Delta) -> Option<M> {
    T::filter_children_hierarchy_change(change)
  }

  fn marking_dirty(
    tree: &mut TreeCollection<DerivedData<T, M>>,
    change_node: TreeNodeHandle<DerivedData<T, M>>,
    dirty_mark: M,
  ) -> Option<TreeNodeHandle<DerivedData<T, M>>> {
    let mut update_parent = None;
    tree
      .create_node_mut_ptr(change_node)
      .traverse_parent_mut(|n| {
        let n = unsafe { &mut (*n.node) };
        if n.parent.is_none() {
          update_parent = n.handle().into();
        }
        let contains = n.data.dirty.contains(&dirty_mark);
        n.data.dirty.insert(&dirty_mark);
        !contains
      });
    update_parent
  }

  fn update_derived(
    source_tree: &TREE,
    derived_tree: &mut TreeCollection<DerivedData<T, M>>,
    update_root: TreeNodeHandle<DerivedData<T, M>>,
    derived_delta_sender: &mut impl FnMut((usize, DeltaPair<T>)),
  ) {
    let node_index = update_root.index();
    let mut node = derived_tree.create_node_mut_ptr(update_root);
    let node_data = &mut unsafe { &mut (*node.node) }.data;
    if node_data.dirty == T::DirtyMark::default() {
      return;
    }

    node_data.dirty = T::DirtyMark::default();

    node.visit_children_mut(|child| {
      let child = unsafe { &mut (*child.node) };
      Self::update_derived(
        source_tree,
        derived_tree,
        child.handle(),
        derived_delta_sender,
      );
    });

    // the source tree maybe out of sync if the other thread mutate the source tree in parallel
    if let Some(source_node) = source_tree.try_recreate_handle(node_index) {
      node_data.data.hierarchy_children_update(
        source_tree.get_node_data(source_node),
        |child_visitor| {
          node.visit_children_mut(|node| {
            let node_data = &unsafe { &mut (*node.node) }.data.data;
            child_visitor(node_data)
          })
        },
        &node_data.dirty,
        |derive, delta| derived_delta_sender((node_index, derive.make_reverse_delta_pair(delta))),
      );
    }
  }
}
