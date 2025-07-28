use std::hash::Hash;

use crate::*;

pub struct MeshletAdjacencyInfo {
  internal: Adjacency<u32>,
}

impl MeshletAdjacencyInfo {
  pub fn build(source: &[EdgeFinder]) -> Self {
    let mut adjacency = Vec::with_capacity(source.len());
    for i in 0..source.len() {
      for j in 0..source.len() {
        if source[i].has_shared_edge(&source[j]) {
          adjacency.push((i as u32, j as u32));
        }
      }
    }

    Self {
      internal: Adjacency::from_iter(
        adjacency.len() * 2,
        adjacency.iter().flat_map(|(i, j)| [*i, *j]),
        adjacency.iter().flat_map(|(i, j)| [(*i, *i), (*j, *i)]),
      ),
    }
  }

  pub fn iter_adjacency_meshlets(&self, meshlet: u32) -> impl Iterator<Item = u32> + '_ {
    self.internal.iter_many_by_one(meshlet).copied()
  }
}

/// un-direct edge
#[derive(Debug, Clone, Copy, Eq)]
pub struct Edge(pub u32, pub u32);
impl PartialEq for Edge {
  fn eq(&self, other: &Self) -> bool {
    (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
  }
}
impl Hash for Edge {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    if self.0 < self.1 {
      self.0.hash(state);
      self.1.hash(state);
    } else {
      self.1.hash(state);
      self.0.hash(state);
    }
  }
}

pub fn compute_locking_edge(
  groups: &[MeshletGroup],
  group_id: u32,
  precompute_meshlet_edges: &[EdgeFinder],
) -> EdgeFinder {
  let group = &groups[group_id as usize];
  let meshlets = precompute_meshlet_edges
    .get(group.meshlets.into_range())
    .unwrap();

  let mut base = meshlets[0].clone();
  for rest in meshlets.get(1..).unwrap() {
    base.merge_from(rest);
  }
  base
}

pub fn compute_all_meshlet_boundary_edges(
  meshlets: &[Meshlet],
  indices: &[u32],
) -> Vec<EdgeFinder> {
  meshlets
    .iter()
    .map(|meshlet| compute_meshlet_boundary_edges(meshlet, indices))
    .collect()
}

pub fn compute_meshlet_boundary_edges(meshlet: &Meshlet, indices: &[u32]) -> EdgeFinder {
  let mut boundary_edges = EdgeFinder::default();

  let indices_range = meshlet.index_range.into_range();
  let indices = indices.get(indices_range).unwrap();

  for [a, b, c] in indices.array_chunks::<3>() {
    boundary_edges.add_edge(*a, *b);
    boundary_edges.add_edge(*b, *c);
    boundary_edges.add_edge(*c, *a);
  }

  boundary_edges
}

#[derive(Default, Clone)]
pub struct EdgeFinder(pub FastHashSet<Edge>);

impl EdgeFinder {
  pub fn add_edge(&mut self, a: u32, b: u32) {
    if !self.0.remove(&Edge(a, b)) {
      self.0.insert(Edge(a, b));
    }
  }
  fn has_shared_edge(&self, other: &Self) -> bool {
    for e in &self.0 {
      if other.0.contains(e) {
        return true;
      }
    }
    false
  }

  fn merge_from(&mut self, other: &Self) {
    for edge in &other.0 {
      self.add_edge(edge.0, edge.1)
    }
  }
}
