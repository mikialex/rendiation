use std::{
  cell::RefCell,
  collections::BTreeSet,
  rc::{Rc, Weak},
};

pub struct Graph<T> {
  nodes: Vec<Rc<RefCell<Node<T>>>>,
}

impl<T> Graph<T> {
  pub fn new() -> Self {
    Self { nodes: Vec::new() }
  }

  pub fn get_node(&self, id: usize) -> WrapNode<T> {
    WrapNode(Rc::downgrade(&self.nodes[id].clone()))
  }

  pub fn topological_order_list(&self, node: &WrapNode<T>) -> Vec<usize> {
    let mut list = Vec::new();
    self.traverse_dfs_in_topological_order(node, &mut |node| list.push(node.id()));
    list
  }

  pub fn traverse_dfs_in_topological_order(
    &self,
    node: &WrapNode<T>,
    visitor: &mut impl FnMut(&WrapNode<T>),
  ) {
    let mut unresolved: BTreeSet<usize> = BTreeSet::new();
    let mut visited: BTreeSet<usize> = BTreeSet::new();

    fn visit<T>(
      n_id: usize,
      visited: &mut BTreeSet<usize>,
      unresolved: &mut BTreeSet<usize>,
      graph: &Graph<T>,
      visitor: &mut impl FnMut(&WrapNode<T>),
    ) {
      if visited.contains(&n_id) {
        return;
      }
      if unresolved.contains(&n_id) {
        panic!("graph contains loops"); // todo
      }

      unresolved.insert(n_id);

      let node = graph.get_node(n_id);
      node.foreach_from(|from_id| visit(from_id, visited, unresolved, graph, visitor));

      unresolved.remove(&n_id);
      visited.insert(n_id);
      visitor(&node)
    }

    visit(node.id(), &mut visited, &mut unresolved, self, visitor);
  }

  pub fn create_node(&mut self, payload: T) -> WrapNode<T> {
    let node = Node {
      id: self.nodes.len(),
      from_target_id: BTreeSet::new(),
      to_target_id: BTreeSet::new(),
      payload,
    };
    let rc = Rc::new(RefCell::new(node));
    self.nodes.push(rc.clone());
    WrapNode(Rc::downgrade(&rc))
  }
}

pub struct Node<T> {
  id: usize,
  from_target_id: BTreeSet<usize>,
  to_target_id: BTreeSet<usize>,
  payload: T,
}

pub struct WrapNode<T>(Weak<RefCell<Node<T>>>);

impl<T> WrapNode<T> {
  pub fn id(&self) -> usize {
    self.0.upgrade().unwrap().borrow().id
  }

  pub fn foreach_from(&self, mut visitor: impl FnMut(usize)) {
    self
      .0
      .upgrade()
      .unwrap()
      .borrow()
      .from_target_id
      .iter()
      .for_each(|id| visitor(*id));
  }

  pub fn connect_to(&self, node: WrapNode<T>) {
    let self_node = self.0.upgrade().unwrap();
    let mut self_node = self_node.borrow_mut();
    let n = node.0.upgrade().unwrap();
    let mut n = n.borrow_mut();
    self_node.to_target_id.insert(n.id);
    n.from_target_id.insert(self_node.id);
  }
}

#[test]
fn test_add() {
  let mut graph = Graph::new();
  let node_a = graph.create_node();
  let node_b = graph.create_node();
  node_a.connect_to(node_b);
  assert_eq!(3, 3);
}
