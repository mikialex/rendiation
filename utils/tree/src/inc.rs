use crate::*;
use ::incremental::*;

#[derive(Clone)]
pub enum TreeMutation<T: IncrementAble> {
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

impl<T: IncrementAble + Clone> IncrementAble for TreeCollection<T> {
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

pub struct TreeCollectionReactiveMutator<'a, T: IncrementAble + Clone> {
  inner: &'a mut TreeCollection<T>,
  collector: &'a mut dyn FnMut(DeltaOf<TreeCollection<T>>),
}

impl<'a, T: IncrementAble + Clone> MutatorApply<TreeCollection<T>>
  for TreeCollectionReactiveMutator<'a, T>
{
  fn apply(&mut self, delta: DeltaOf<TreeCollection<T>>) {
    self.inner.apply(delta).unwrap()
  }
}

impl<'a, T: IncrementAble + Clone> TreeCollectionReactiveMutator<'a, T> {
  pub fn create(&mut self, node: T) -> TreeNodeHandle<T> {
    (self.collector)(TreeMutation::Create(node.clone()));
    self.inner.create_node(node)
  }

  pub fn get_node_mut(
    &'a mut self,
    node: TreeNodeHandle<T>,
  ) -> (impl FnMut(DeltaOf<T>) + 'a, T::Mutator<'a>) {
    let mut collector =
      |delta| (self.collector)(DeltaOf::<TreeCollection<T>>::Mutate { node, delta });
    (
      collector,
      self
        .inner
        .get_node_mut(node)
        .data_mut()
        .create_mutator(&mut collector),
    )
  }
}
