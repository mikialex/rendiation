use fast_hash_collection::FastHashMap;
use rendiation_geometry::Triangle;

use crate::*;

impl<V> FromIterator<Triangle<V>> for IndexedMesh<TriangleList, Vec<V>, Vec<u32>>
where
  V: std::hash::Hash + Eq + Copy,
{
  fn from_iter<T: IntoIterator<Item = Triangle<V>>>(iter: T) -> Self {
    let mut deduplicate = FastHashMap::<V, u32>::default();
    let iter = iter.into_iter();

    let mut vertices: Vec<V> = Vec::with_capacity(iter.size_hint().0 * 3);
    let mut indices: Vec<u32> = Vec::with_capacity(iter.size_hint().0 * 3);

    let mut push_v = |v: V| {
      *deduplicate.entry(v).or_insert_with(|| {
        vertices.push(v);
        vertices.len() as u32 - 1
      })
    };
    iter.for_each(|tri| {
      indices.push(push_v(tri.a));
      indices.push(push_v(tri.b));
      indices.push(push_v(tri.c));
    });

    vertices.shrink_to_fit();
    indices.shrink_to_fit();
    IndexedMesh::new(vertices, indices)
  }
}

impl<V> FromIterator<Triangle<V>> for NoneIndexedMesh<TriangleList, Vec<V>> {
  fn from_iter<T: IntoIterator<Item = Triangle<V>>>(iter: T) -> Self {
    let iter = iter.into_iter();

    let mut vertices: Vec<V> = Vec::with_capacity(iter.size_hint().0 * 3);

    iter.for_each(|tri| {
      vertices.push(tri.a);
      vertices.push(tri.b);
      vertices.push(tri.c);
    });

    vertices.shrink_to_fit();
    NoneIndexedMesh::new(vertices)
  }
}
