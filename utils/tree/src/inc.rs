use crate::*;
use ::incremental::*;

#[derive(Clone)]
pub enum TreeMutation<T: Incremental> {
  Create(T),
  Delete(TreeNodeHandle<T>),
  Mutate {
    node: TreeNodeHandle<T>,
    delta: T::Delta,
  },
  Attach {
    parent_target: TreeNodeHandle<T>,
    node: TreeNodeHandle<T>,
  },
  Detach {
    node: TreeNodeHandle<T>,
  },
}

impl<T: Incremental + Clone> Incremental for TreeCollection<T> {
  type Delta = TreeMutation<T>;
  type Error = ();

  type Mutator<'a> = TreeCollectionReactiveMutator<'a, T>
  where
    Self: 'a;

  #[allow(clippy::needless_lifetimes)]
  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    TreeCollectionReactiveMutator {
      inner: self,
      collector,
    }
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      TreeMutation::Create(d) => {
        self.create_node(d);
      }
      TreeMutation::Delete(d) => self.delete_node(d),
      TreeMutation::Mutate { node, delta } => {
        let node = self.get_node_mut(node).data_mut();
        node.apply(delta).unwrap();
      }
      TreeMutation::Attach {
        parent_target,
        node,
      } => self.node_add_child_by(parent_target, node).unwrap(),
      TreeMutation::Detach { node } => {
        self.node_detach_parent(node).unwrap();
      }
    }
    Ok(())
  }

  fn expand(&self, _cb: impl FnMut(Self::Delta)) {
    todo!()
  }
}

pub struct TreeCollectionReactiveMutator<'a, T: Incremental + Clone> {
  inner: &'a mut TreeCollection<T>,
  collector: &'a mut dyn FnMut(DeltaOf<TreeCollection<T>>),
}

impl<'a, T: Incremental + Clone> MutatorApply<TreeCollection<T>>
  for TreeCollectionReactiveMutator<'a, T>
{
  fn apply(&mut self, delta: DeltaOf<TreeCollection<T>>) {
    self.inner.apply(delta).unwrap()
  }
}

impl<'a, T: Incremental + Clone> TreeCollectionReactiveMutator<'a, T> {
  pub fn create(&mut self, node: T) -> TreeNodeHandle<T> {
    (self.collector)(TreeMutation::Create(node.clone()));
    self.inner.create_node(node)
  }

  pub fn get_node_mut(
    &'a mut self,
    node: TreeNodeHandle<T>,
  ) -> MutateMapper<'a, T, impl FnMut(T::Delta) + 'a> {
    let t = self.inner.get_node_mut(node).data_mut();
    let collector = &mut self.collector;
    let collector = move |delta| collector(DeltaOf::<TreeCollection<T>>::Mutate { node, delta });

    MutateMapper {
      inner: t,
      collector,
    }
  }
}

#[test]
fn test_nested_mutator() {
  let mut tree = TreeCollection::<usize>::default();
  let mut tree_collector = |_| {};
  let root = {
    let mut tree_mutator = tree.create_mutator(&mut tree_collector);
    let root = tree_mutator.create(1);
    let mut node_mutation_mapper = tree_mutator.get_node_mut(root);
    let mut node_mutator = node_mutation_mapper.mutate();
    node_mutator.apply(2);
    root
  };

  let tree_root_raw = tree.get_node(root).data();
  assert_eq!(*tree_root_raw, 2);
}

pub struct MutateMapper<'a, T: Incremental, C: FnMut(T::Delta) + 'a> {
  inner: &'a mut T,
  collector: C,
}

impl<'a, T, C> MutateMapper<'a, T, C>
where
  T: Incremental,
  C: FnMut(T::Delta) + 'a,
{
  pub fn mutate(&'a mut self) -> T::Mutator<'a> {
    self.inner.create_mutator(&mut self.collector)
  }
}
