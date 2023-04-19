use std::ops::Deref;

use futures::{Stream, StreamExt};
use incremental::IncrementalBase;
use reactive::{do_updates, SignalStreamExt, StreamForker};

use crate::*;

/// The default value is the none parent case
///
/// We not impose IncrementalHierarchyDerived extends HierarchyDerived
/// because of simplicity.
pub trait IncrementalHierarchyDerived: Default + IncrementalBase {
  type Source: IncrementalBase;
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
    collect: impl FnMut(Self::Delta),
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

pub trait IncrementalChildrenHierarchyDerived: Default + IncrementalBase {
  type Source: IncrementalBase;
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
    collect: impl FnMut(Self::Delta),
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

pub struct TreeHierarchyDerivedSystem<T: IncrementalBase, Dirty> {
  derived_tree: Arc<RwLock<TreeCollection<DerivedData<T, Dirty>>>>,
  // we use boxed here to avoid another generic for tree delta input stream
  pub derived_stream: StreamForker<Box<dyn Stream<Item = (usize, T::Delta)> + Unpin>>,
}

impl<T: IncrementalBase, M> Clone for TreeHierarchyDerivedSystem<T, M> {
  fn clone(&self) -> Self {
    Self {
      derived_tree: self.derived_tree.clone(),
      derived_stream: self.derived_stream.clone(),
    }
  }
}

impl<T: IncrementalBase, Dirty> TreeHierarchyDerivedSystem<T, Dirty> {
  pub fn maintain(&mut self) {
    do_updates(&mut self.derived_stream, |_| {});
  }

  pub fn visit_derived_tree<R>(
    &self,
    mut v: impl FnMut(&TreeCollection<DerivedData<T, Dirty>>) -> R,
  ) -> R {
    v(&self.derived_tree.read().unwrap())
  }

  pub fn new<B, TREE, S, M>(
    tree_delta: impl Stream<Item = TreeMutation<S>> + 'static,
    source_tree: &SharedTreeCollection<TREE>,
  ) -> TreeHierarchyDerivedSystem<T, B::Dirty>
  where
    B: TreeIncrementalDeriveBehavior<T, S, M, TREE, Dirty = Dirty>,
    S: IncrementalBase,
    T: Default,
    Dirty: Default + 'static,
    M: HierarchyDirtyMark,
    TREE: 'static,
  {
    let derived_tree = Arc::new(RwLock::new(
      TreeCollection::<DerivedData<T, B::Dirty>>::default(),
    ));

    let derived_tree_c = derived_tree.clone();
    let derived_tree_cc = derived_tree_c.clone();

    let source_tree = source_tree.clone();

    let derived_stream = tree_delta
      // mark stage
      // do dirty marking, return if should trigger hierarchy change, and the update root
      .filter_map_sync(move |delta| {
        let mut derived_tree = derived_tree_c.write().unwrap();
        match delta {
          // simply create the default derived. insert into derived tree.
          // we don't care the returned handle, as we assume they are allocated in the same position
          // in the original tree.
          TreeMutation::Create { .. } => {
            let node = derived_tree.create_node(Default::default());
            B::marking_dirty(&mut derived_tree, node, M::all_dirty())
          }
          // do pair remove in derived tree
          TreeMutation::Delete(handle) => {
            let handle = derived_tree.recreate_handle(handle);
            derived_tree.delete_node(handle);
            None
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
        }
      })
      .buffered_unbound() // to make sure all markup finished
      .map(move |update_root| {
        let mut derived_tree = derived_tree_cc.write().unwrap();
        // this allocation can not removed, but could we calculate correct capacity or reuse the allocation?
        let mut derived_deltas = Vec::new();
        // do full tree traverse check, emit all real update as stream
        let tree: &TREE = &source_tree.inner.read().unwrap();
        // node maybe deleted
        if derived_tree.is_handle_valid(update_root) {
          B::update_derived(tree, &mut derived_tree, update_root, &mut |delta| {
            derived_deltas.push(delta);
          });
        }
        futures::stream::iter(derived_deltas)
      })
      .flatten(); // we want all change here, not signal

    let boxed: Box<dyn Stream<Item = (usize, T::Delta)> + Unpin> =
      Box::new(Box::pin(derived_stream));

    Self {
      derived_tree,
      derived_stream: boxed.create_board_caster(),
    }
  }
}

pub trait TreeIncrementalDeriveBehavior<T: IncrementalBase, S: IncrementalBase, M, TREE> {
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
    derived_delta_sender: &mut impl FnMut((usize, T::Delta)),
  );
}

pub struct ParentTree;

impl<T: IncrementalBase, M, TREE, X> TreeIncrementalDeriveBehavior<T, T::Source, M, TREE>
  for ParentTree
where
  T: IncrementalHierarchyDerived<DirtyMark = M>,
  M: HierarchyDirtyMark,
  TREE: CoreTree<Node = X>,
  X: Deref<Target = T::Source>,
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
    derived_delta_sender: &mut impl FnMut((usize, T::Delta)),
  ) {
    derived_tree.traverse_mut_pair(update_root, |node, parent| {
      let node_index = node.handle().index();
      let derived = node.data_mut();

      if derived.dirty.sub_tree_dirty_mark_any == T::DirtyMark::default() {
        NextTraverseVisit::SkipChildren
      } else {
        let parent = parent.map(|parent| &parent.data().data);

        let source_node = source_tree.recreate_handle(node_index);
        let source_node = source_tree.get_node_data(source_node).deref();

        derived.data.hierarchy_update(
          source_node,
          parent,
          &derived.dirty.sub_tree_dirty_mark_all,
          |delta| derived_delta_sender((node_index, delta)),
        );

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

impl<T: IncrementalBase, M, TREE, X> TreeIncrementalDeriveBehavior<T, T::Source, M, TREE>
  for ChildrenTree
where
  T: IncrementalChildrenHierarchyDerived<DirtyMark = M>,
  M: HierarchyDirtyMark,
  TREE: CoreTree<Node = X>,
  X: Deref<Target = T::Source>,
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
        // don't check self, or we will failed to mark change
        if n.handle() == change_node {
          return true;
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
    derived_delta_sender: &mut impl FnMut((usize, <T as IncrementalBase>::Delta)),
  ) {
    let node_index = update_root.index();
    let mut node = derived_tree.create_node_mut_ptr(update_root);
    let node_data = &mut unsafe { &mut (*node.node) }.data;
    if node_data.dirty == T::DirtyMark::default() {
      return;
    }

    node_data.dirty = T::DirtyMark::default();

    let source_node = source_tree.recreate_handle(node_index);
    let source_node = source_tree.get_node_data(source_node).deref();

    node.visit_children_mut(|child| {
      let child = unsafe { &mut (*child.node) };
      Self::update_derived(
        source_tree,
        derived_tree,
        child.handle(),
        derived_delta_sender,
      );
    });

    node_data.data.hierarchy_children_update(
      source_node,
      |child_visitor| {
        node.visit_children_mut(|node| {
          let node_data = &unsafe { &mut (*node.node) }.data.data;
          child_visitor(node_data)
        })
      },
      &node_data.dirty,
      &mut |delta| derived_delta_sender((node_index, delta)),
    );
  }
}
