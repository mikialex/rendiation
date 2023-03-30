use std::ops::Deref;

use futures::{Stream, StreamExt};
use incremental::IncrementalBase;
use reactive::{SignalStreamExt, StreamForker};

use crate::*;

pub trait HierarchyDepend: Sized {
  fn update_by_parent(&mut self, parent: Option<&Self>);
}

impl<T: HierarchyDepend, X: IncrementalBase> SharedTreeCollection<ReactiveTreeCollection<T, X>> {
  pub fn update(&self, root: TreeNodeHandle<T>) {
    let mut nodes = self.inner.write().unwrap();
    nodes.inner.traverse_mut_pair(root, |this, parent| {
      let parent = parent.map(|parent| parent.data());
      let node_data = this.data_mut();
      node_data.update_by_parent(parent);
      NextTraverseVisit::VisitChildren
    });
  }
}

pub trait IncrementalHierarchyDepend: HierarchyDepend + IncrementalBase {
  fn check_delta_affects_hierarchy(delta: &Self::Delta) -> bool;
}

/// The default value is the none parent case
pub trait HierarchyDerived: Default + IncrementalBase {
  type Source: IncrementalBase;
  type HierarchyDirtyMark: HierarchyDirtyMark;

  /// for any delta of source, check if it will have hierarchy effect
  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::HierarchyDirtyMark>;

  fn hierarchy_update(
    &mut self,
    self_source: &Self::Source,
    parent_derived: Option<&Self>,
    dirty: &Self::HierarchyDirtyMark,
    collect: impl FnMut(Self::Delta),
  );
  // just shortcut
  fn full_dirty_mark() -> Self::HierarchyDirtyMark {
    Self::HierarchyDirtyMark::all_dirty()
  }
}

/// Default should use None state
pub trait HierarchyDirtyMark: PartialEq + Default {
  fn contains(&self, mark: &Self) -> bool;
  fn intersects(&self, mark: &Self) -> bool;
  fn insert(&mut self, mark: &Self);
  fn all_dirty() -> Self;
}

#[derive(Default)]
pub struct DerivedData<T: HierarchyDerived> {
  pub data: T,
  /// all sub tree change or together includes self
  /// this tag is for skip tree branch updating
  sub_tree_dirty_mark_any: T::HierarchyDirtyMark,

  /// all sub tree change and together includes self.
  /// this tag is for skipping tree dirty marking
  sub_tree_dirty_mark_all: T::HierarchyDirtyMark,
}

pub struct TreeHierarchyDerivedSystem<T: HierarchyDerived> {
  derived_tree: Arc<RwLock<TreeCollection<DerivedData<T>>>>,
  // we use boxed here to avoid another generic for tree delta input stream
  pub derived_stream: StreamForker<Box<dyn Stream<Item = (usize, T::Delta)> + Unpin>>,
}

impl<T> TreeHierarchyDerivedSystem<T>
where
  T: HierarchyDerived,
  T::Source: IncrementalBase,
{
  pub fn visit_derived_tree<R>(
    &self,
    mut v: impl FnMut(&TreeCollection<DerivedData<T>>) -> R,
  ) -> R {
    v(&self.derived_tree.read().unwrap())
  }

  pub fn new<TREE, X>(
    tree_delta: impl Stream<Item = TreeMutation<T::Source>> + 'static,
    source_tree: &SharedTreeCollection<TREE>,
  ) -> Self
  where
    TREE: CoreTree<Node = X> + 'static,
    X: Deref<Target = T::Source>,
  {
    let derived_tree = Arc::new(RwLock::new(TreeCollection::<DerivedData<T>>::default()));

    let derived_tree_c = derived_tree.clone();
    let derived_tree_cc = derived_tree_c.clone();

    let source_tree = source_tree.clone();

    let derived_stream = tree_delta
      // mark stage
      // do dirty marking, return if should trigger hierarchy change, and the update root
      .filter_map(move |delta| {
        let derived_tree_ccc = derived_tree_c.clone(); // have to move step by step
        async move {
          let mut derived_tree = derived_tree_ccc.write().unwrap();
          match delta {
            // simply create the default derived. insert into derived tree.
            // we don't care the returned handle, as we assume they are allocated in the same position
            // in the original tree.
            TreeMutation::Create { .. } => {
              derived_tree.create_node(Default::default());
              None
            }
            // do pair remove in derived tree
            TreeMutation::Delete(handle) => {
              let handle = derived_tree.recreate_handle(handle);
              derived_tree.delete_node(handle);
              None
            }
            // check if have any hierarchy effect
            // for children, do traverse mark dirty all-mark, skip if all-mark contains new dirty
            // for parent chain, do parent chain traverse mark dirty any-mark,
            TreeMutation::Mutate { node, delta } => {
              let handle = derived_tree.recreate_handle(node);
              T::filter_hierarchy_change(&delta).and_then(|dirty_mark| {
                mark_sub_tree_full_change(&mut derived_tree, handle, dirty_mark)
              })
            }
            // like update, and we will emit full dirty change
            TreeMutation::Attach {
              parent_target,
              node,
            } => {
              let parent_target = derived_tree.recreate_handle(parent_target);
              let node = derived_tree.recreate_handle(node);
              derived_tree.node_add_child_by(parent_target, node).ok();
              mark_sub_tree_full_change(&mut derived_tree, node, T::full_dirty_mark())
            }
            // ditto
            TreeMutation::Detach { node } => {
              let node = derived_tree.recreate_handle(node);
              derived_tree.node_detach_parent(node).ok();
              mark_sub_tree_full_change(&mut derived_tree, node, T::full_dirty_mark())
            }
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
        do_sub_tree_updates(tree, &mut derived_tree, update_root, &mut |delta| {
          derived_deltas.push(delta);
        });
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

// return the root node handle as the update root
// if the parent chain has been any dirty marked, we return None to skip the update process
fn mark_sub_tree_full_change<T: HierarchyDerived>(
  tree: &mut TreeCollection<DerivedData<T>>,
  change_node: TreeNodeHandle<DerivedData<T>>,
  dirty_mark: T::HierarchyDirtyMark,
) -> Option<TreeNodeHandle<DerivedData<T>>> {
  tree
    .create_node_mut_ptr(change_node)
    .traverse_iter_mut(|node| {
      let child = unsafe { &(*node.node) };
      if child.data.sub_tree_dirty_mark_all.contains(&dirty_mark) {
        NextTraverseVisit::SkipChildren
      } else {
        NextTraverseVisit::VisitChildren
      }
    })
    .for_each(|node| {
      let child = unsafe { &mut (*node.node) };
      child.data.sub_tree_dirty_mark_all.insert(&dirty_mark);
      child.data.sub_tree_dirty_mark_any.insert(&dirty_mark);
    });

  let mut update_parent = None;
  tree
    .create_node_mut_ptr(change_node)
    .traverse_parent_mut(|parent| {
      let parent = unsafe { &mut (*parent.node) };
      if parent.parent.is_none() {
        update_parent = parent.handle().into();
      }
      let should_pop = parent.data.sub_tree_dirty_mark_any.contains(&dirty_mark);
      parent.data.sub_tree_dirty_mark_any.insert(&dirty_mark);
      should_pop
    });

  update_parent
}

fn do_sub_tree_updates<T, TREE, X>(
  source_tree: &TREE,
  derived_tree: &mut TreeCollection<DerivedData<T>>,
  update_root: TreeNodeHandle<DerivedData<T>>,
  derived_delta_sender: &mut impl FnMut((usize, T::Delta)),
) where
  T: HierarchyDerived,
  TREE: CoreTree<Node = X>,
  X: Deref<Target = T::Source>,
{
  derived_tree.traverse_mut_pair(update_root, |node, parent| {
    let node_index = node.handle().index();
    let derived = node.data_mut();

    if derived.sub_tree_dirty_mark_any == T::HierarchyDirtyMark::default() {
      NextTraverseVisit::SkipChildren
    } else {
      let parent = parent.map(|parent| &parent.data().data);

      let source_node = source_tree.recreate_handle(node_index);
      let source_node = source_tree.get_node_data(source_node).deref();

      derived.data.hierarchy_update(
        source_node,
        parent,
        &derived.sub_tree_dirty_mark_any,
        |delta| derived_delta_sender((node_index, delta)),
      );

      derived.sub_tree_dirty_mark_any = T::HierarchyDirtyMark::default();
      derived.sub_tree_dirty_mark_all = T::HierarchyDirtyMark::default();

      NextTraverseVisit::VisitChildren
    }
  })
}
