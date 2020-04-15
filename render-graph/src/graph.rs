use std::{
  cell::RefCell,
  collections::BTreeSet,
  rc::{Rc, Weak},
};

pub struct Graph {
  nodes: Vec<Rc<RefCell<Node>>>,
}

impl Graph {
  pub fn new() -> Self {
    Self {
      nodes: Vec::new(),
    }
  }

  pub fn build(&mut self, root: WrapNode) {}

  pub fn get_node(&self, id: usize) -> WrapNode {
    WrapNode(Rc::downgrade(&self.nodes[id].clone()))
  }

  pub fn traverse_dfs(
    &self,
    node: &WrapNode,
    mut visitor: impl FnMut(&WrapNode),
  ) -> Result<(), String> {
    let mut visited: BTreeSet<usize> = BTreeSet::new();

    let mut nodes = Vec::new();
    nodes.push(node.id());

    while let Some(n_id) = nodes.pop() {
      let node = self.get_node(n_id);
      if !visited.contains(&node.id()) {
        visited.insert(node.id());
        visitor(&node);

        node.foreach_from(|from_id| nodes.push(from_id));
        visited.remove(&node.id());
      } else {
        return Err(String::from("node graph contains cycles."));
      }
    }

    Ok(())
  }

  // getAllDependency(): Set<DAGNode>{
  //   const result: Set<DAGNode> = new Set();
  //   this.traverseDFS((n) => {
  //     result.add(n);
  //   })
  //   return result;
  // }

  pub fn create_node(&mut self) -> WrapNode {
    let node = Node {
      id: self.nodes.len(),
      from_target_id: BTreeSet::new(),
      to_target_id: BTreeSet::new(),
    };
    let rc = Rc::new(RefCell::new(node));
    self.nodes.push(rc.clone());
    WrapNode(Rc::downgrade(&rc))
  }
}

pub struct Node {
  id: usize,
  from_target_id: BTreeSet<usize>,
  to_target_id: BTreeSet<usize>,
}

pub struct WrapNode(Weak<RefCell<Node>>);

impl WrapNode {
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

  pub fn connect_to(&self, node: WrapNode) {
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
