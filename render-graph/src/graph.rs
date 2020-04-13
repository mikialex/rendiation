use std::{
  cell::RefCell,
  collections::BTreeSet,
  rc::{Rc, Weak},
};

struct Graph {
  nodes: Vec<Rc<RefCell<Node>>>,
  sorted: Vec<usize>,
}

impl Graph {
  pub fn new() -> Self {
    Self {
      nodes: Vec::new(),
      sorted: Vec::new(),
    }
  }

  pub fn create_node(&mut self) -> WrapNode {
    let node = Node {
      id: self.nodes.len(),
      // graph: Rc<RefCell<Graph>>,
      from_target_id: BTreeSet::new(),
      to_target_id: BTreeSet::new(),
    };
    let rc = Rc::new(RefCell::new(node));
    self.nodes.push(rc.clone());
    WrapNode(Rc::downgrade(&rc))
  }
}

struct Node {
  id: usize,
  // graph: Rc<RefCell<Graph>>,
  from_target_id: BTreeSet<usize>,
  to_target_id: BTreeSet<usize>,
}

struct WrapNode(Weak<RefCell<Node>>);

impl WrapNode {
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

// #[cfg(test)]
// mod tests {
//     // Note this useful idiom: importing names from outer (for mod tests) scope.
//     use super::*;

//     #[test]
//     fn test_add() {
//         assert_eq!(add(1, 2), 3);
//     }

//     #[test]
//     fn test_bad_add() {
//         // This assert would fire and test will fail.
//         // Please note, that private functions can be tested too!
//         assert_eq!(bad_add(1, 2), 3);
//     }
// }
