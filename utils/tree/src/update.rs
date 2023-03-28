use futures::{Stream, StreamExt};
use incremental::IncrementalBase;
use reactive::SignalStreamExt;

use crate::*;

pub trait HierarchyDepend: Sized {
  fn update_by_parent(&mut self, parent: Option<&Self>);
}

impl<T: HierarchyDepend> SharedTreeCollection<TreeCollection<T>> {
  pub fn update(&self, root: TreeNodeHandle<T>) {
    let mut nodes = self.inner.write().unwrap();
    nodes.traverse_mut_pair(root, |this, parent| {
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

  fn hierarchy_update(&self, parent: Option<&Self::Source>, collect: &mut impl FnMut(Self::Delta));
  // just shortcut
  fn full_dirty_mark() -> Self::HierarchyDirtyMark {
    Self::HierarchyDirtyMark::ALL
  }
}

/// Default should use None state
pub trait HierarchyDirtyMark: PartialEq + Default {
  const ALL: Self;
  fn contains(&self, mark: &Self) -> bool;
  fn intersects(&self, mark: &Self) -> bool;
  fn insert(&mut self, mark: &Self);
}

#[derive(Default)]
struct DerivedData<T: HierarchyDerived> {
  data: T,
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
  derived_stream: Box<dyn Stream<Item = T::Delta> + Unpin>,
}

impl<T> TreeHierarchyDerivedSystem<T>
where
  T: HierarchyDerived,
  T::Source: IncrementalBase,
{
  #[allow(unused_must_use)]
  pub fn new(
    tree_delta: impl Stream<Item = TreeMutation<T::Source>> + 'static,
    source_tree: &SharedTreeCollection<TreeCollection<T::Source>>,
  ) -> Self {
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
              derived_tree.node_add_child_by(parent_target, node);
              mark_sub_tree_full_change(&mut derived_tree, node, T::full_dirty_mark())
            }
            // ditto
            TreeMutation::Detach { node } => {
              let node = derived_tree.recreate_handle(node);
              derived_tree.node_detach_parent(node);
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
        do_sub_tree_updates(
          &source_tree.inner.read().unwrap(),
          &mut derived_tree,
          update_root,
          &mut |delta| {
            derived_deltas.push(delta);
          },
        );
        futures::stream::iter(derived_deltas)
      })
      .flatten();

    Self {
      derived_tree,
      derived_stream: Box::new(Box::pin(derived_stream)),
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

  // tree.traverse_mut_parent_chain_iter(change_node);

  todo!()
}

fn do_sub_tree_updates<T: HierarchyDerived>(
  source_tree: &TreeCollection<T::Source>,
  derived_tree: &mut TreeCollection<DerivedData<T>>,
  update_root: TreeNodeHandle<DerivedData<T>>,
  derived_delta_sender: &mut impl FnMut(T::Delta),
) {
  derived_tree.traverse_mut(update_root, |node| {
    let parent = node.parent;
    let derived = node.data_mut();

    if derived.sub_tree_dirty_mark_any == T::HierarchyDirtyMark::default() {
      NextTraverseVisit::SkipChildren
    } else {
      derived.sub_tree_dirty_mark_any = T::HierarchyDirtyMark::default();
      derived.sub_tree_dirty_mark_all = T::HierarchyDirtyMark::default();

      let parent = parent
        .map(|parent| source_tree.recreate_handle(parent.index()))
        .map(|handle| source_tree.get_node(handle).data());

      derived.data.hierarchy_update(parent, derived_delta_sender);

      NextTraverseVisit::VisitChildren
    }
  })
}
