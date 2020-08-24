struct HierarchyComponent {
  parent: usize,
}

struct HierachyManager{
  roots: Vec<usize>,
  children_cache: Vec<Vec<usize>>,
  changing: Vec<usize>,
}