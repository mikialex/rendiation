use std::hash::Hash;

use crate::*;

pub struct MeshletAdjacencyInfo {
  pub adjacent_meshlets: Vec<OffsetSize>,
  pub adjacent_meshlets_idx: Vec<u32>,
}

impl MeshletAdjacencyInfo {
  pub fn build(source: &[EdgeFinder]) -> Self {
    let mut adjacency = Vec::with_capacity(source.len());
    for i in 0..source.len() {
      for j in 0..source.len() {
        if i >= j {
          // connectivity is symmetrical
          if source[i].has_shared_edge(&source[j]) {
            adjacency.push((i, j));
          }
        }
      }
    }

    let mut counts = vec![0; source.len()];
    for (i, j) in &adjacency {
      counts[*i] += 1;
      counts[*j] += 1;
    }
    let mut offsets = vec![0; source.len()];
    let mut offset = 0;
    for (i, count) in counts.iter().enumerate() {
      offsets[i] = offset;
      offset += count;
    }

    let mut adjacent_meshlets_idx = vec![0; offset as usize];
    for (i, j) in adjacency {
      adjacent_meshlets_idx[offsets[i] as usize] = j as u32;
      offsets[i] += 1;
      adjacent_meshlets_idx[offsets[j] as usize] = i as u32;
      offsets[j] += 1;
    }

    // fix offsets that have been disturbed by the previous pass
    for (offset, count) in offsets.iter_mut().zip(counts.iter()) {
      assert!(*offset >= *count);
      *offset -= *count;
    }

    Self {
      adjacent_meshlets: offsets
        .into_iter()
        .zip(counts)
        .map(|(offset, size)| OffsetSize { offset, size })
        .collect(),
      adjacent_meshlets_idx,
    }
  }

  pub fn iter_adjacency_meshlets(&self, meshlet: u32) -> impl Iterator<Item = u32> + '_ {
    self
      .adjacent_meshlets_idx
      .get(self.adjacent_meshlets[meshlet as usize].into_range())
      .unwrap()
      .iter()
      .cloned()
  }

  pub fn update_by_remove_a_meshlet(&mut self, target_meshlet_idx: u32, remove_meshlet_idx: u32) {
    todo!()
  }
}

/// un-direct edge
#[derive(Debug, Clone, Copy, Eq)]
pub struct Edge(u32, u32);
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
  let meshlet_boundary_edges: Vec<_> = (0..meshlets.len())
    .map(|meshlet| compute_meshlet_boundary_edges(meshlets, indices, meshlet as u32))
    .collect();
  //

  meshlet_boundary_edges
}
pub fn compute_meshlet_boundary_edges(
  meshlets: &[Meshlet],
  indices: &[u32],
  meshlet: u32,
) -> EdgeFinder {
  let meshlet = meshlets[meshlet as usize];

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
pub struct EdgeFinder(FastHashSet<Edge>);

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
