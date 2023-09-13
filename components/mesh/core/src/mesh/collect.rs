use fast_hash_collection::FastHashMap;
use rendiation_geometry::{LineSegment, Point, Triangle};

use crate::*;

// we should consider merge it with other similar trait
pub trait Simplex: IntoIterator<Item = Self::Vertex> {
  type Vertex;
  type Topology;
  const DIMENSION: usize;
}

impl<V> Simplex for Point<V> {
  type Vertex = V;
  type Topology = PointList;
  const DIMENSION: usize = 1;
}
impl<V> Simplex for LineSegment<V> {
  type Vertex = V;
  type Topology = LineList;
  const DIMENSION: usize = 2;
}
impl<V> Simplex for Triangle<V> {
  type Vertex = V;
  type Topology = TriangleList;
  const DIMENSION: usize = 3;
}

impl<P: Simplex> FromIterator<P> for NoneIndexedMesh<P::Topology, Vec<P::Vertex>> {
  fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
    let iter = iter.into_iter();
    NoneIndexedMesh::new(iter.flatten().collect())
  }
}

impl<P: Simplex> FromIterator<P> for IndexedMesh<P::Topology, Vec<P::Vertex>, Vec<u32>>
where
  P::Vertex: std::hash::Hash + Eq + Copy,
{
  fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
    let mut deduplicate = FastHashMap::<P::Vertex, u32>::default();
    let iter = iter.into_iter();

    let mut vertices: Vec<P::Vertex> = Vec::with_capacity(iter.size_hint().0 * P::DIMENSION);

    let push_v = |v: P::Vertex| {
      *deduplicate.entry(v).or_insert_with(|| {
        vertices.push(v);
        vertices.len() as u32 - 1
      })
    };

    let indices = iter.flat_map(|p| p.into_iter()).map(push_v).collect();
    vertices.shrink_to_fit();

    IndexedMesh::new(vertices, indices)
  }
}
