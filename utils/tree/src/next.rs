pub struct TreeNode<T> {
  parent: IncrementalSignalWeakPtr<Self>,
  data: T,
}

pub trait TreeStorageExt {
  fn listen_sub_tree_connection(&self) -> SubTreeConnectionBuffer;
}

impl<T: IncrementalBase> IncrementalSignalStorage<TreeNode<T>> {
  fn listen_sub_tree_connection(&self) -> SubTreeConnectionBuffer {
    todo!()
  }
}

pub struct SubTreeConnectionBuffer {
  connection: Vec<usize>,
}

struct TraverseCtx<'a, T> {
  nodes: &'a IncrementalSignalStorageImpl<TreeNode<T>>,
  connection: &'a SubTreeConnectionBuffer,
}

struct TreeNodeTraverse<'a, T> {
  ctx: TraverseCtx<'a, T>,
  node: AllocIdx<TreeNode<T>>,
}

impl<'a, T> AbstractTreeNode for TreeNodeTraverse<'a, T> {
  //
}
