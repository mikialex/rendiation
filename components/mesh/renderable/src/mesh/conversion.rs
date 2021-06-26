//! The conversion method between different mesh types

// todo for convert between different mesh type

// downgrade:
// mesh -> line , wireframe? edge?
// line -> point

// indexed -> noneIndexed expand?
// noneIndexed -> indexed indexed?

use super::{
  AbstractIndexMesh, AbstractMesh, HashAbleByConversion, IndexPrimitiveTopologyMeta, IndexType,
  IndexedMesh, IndexedPrimitiveData, LineList, MeshDataContainer, NoneIndexedMesh, PointList,
  PrimitiveTopologyMeta,
};
use rendiation_algebra::{InnerProductSpace, Vec3};
use rendiation_geometry::{LineSegment, Triangle};
use std::{
  cmp::Ordering,
  collections::{HashMap, HashSet},
  ops::Deref,
};

impl<I, V, T, U> IndexedMesh<I, V, T, U>
where
  I: IndexType,
  T: IndexPrimitiveTopologyMeta<I, V, Primitive = Triangle<V>>,
  U: MeshDataContainer<V>,
  V: Deref<Target = Vec3<f32>> + Copy,
{
  pub fn create_wireframe(&self) -> IndexedMesh<I, V, LineList, U> {
    let mut deduplicate_set = HashSet::<LineSegment<I>>::new();
    self
      .primitive_iter()
      .zip(self.index_primitive_iter())
      .for_each(|(_, f)| {
        f.for_each_edge(|edge| {
          deduplicate_set.insert(edge.swap_if(|l| l.start < l.end));
        })
      });
    let new_index = deduplicate_set
      .iter()
      .flat_map(|l| l.iter_point())
      .collect();
    IndexedMesh::<I, V, LineList, U>::new(self.data.clone(), new_index)
  }

  /// maybe you should merge vertex before create edge
  /// non manifold mesh may affect result
  pub fn create_edge(&self, edge_threshold_angle: f32) -> NoneIndexedMesh<V, LineList, U> {
    // Map: edge id => (edge face idA, edge face idB(optional));
    let mut edges = HashMap::<LineSegment<I>, (usize, Option<usize>)>::new();
    self
      .primitive_iter()
      .zip(self.index_primitive_iter())
      .enumerate()
      .for_each(|(face_id, (_, f))| {
        f.for_each_edge(|edge| {
          edges
            .entry(edge.swap_if(|l| l.start < l.end))
            .and_modify(|e| e.1 = Some(face_id))
            .or_insert_with(|| (face_id, None));
        })
      });
    let normals = self
      .primitive_iter()
      .map(|f| f.map(|v| *v).face_normal().value)
      .collect::<Vec<Vec3<f32>>>();
    let threshold_dot = edge_threshold_angle.cos();
    let data = edges
      .iter()
      .filter(|(_, f)| f.1.is_none() || normals[f.0].dot(normals[f.1.unwrap()]) <= threshold_dot)
      .map(|(e, _)| e)
      .flat_map(|l| l.iter_point())
      .map(|i| self.data.as_ref()[i.into_usize()])
      .collect();
    NoneIndexedMesh::new(data)
  }
}

impl<I, V, T> IndexedMesh<I, V, T>
where
  I: IndexType,
  T: IndexPrimitiveTopologyMeta<I, V>,
  V: Copy,
  <T as PrimitiveTopologyMeta<V>>::Primitive: IndexedPrimitiveData<I, V, Vec<V>, Vec<I>>,
{
  pub fn merge_vertex_by_sorting(
    &self,
    mut sorter: impl FnMut(&V, &V) -> Ordering,
    mut merger: impl FnMut(&V, &V) -> bool,
  ) -> IndexedMesh<I, V, T> {
    let mut resorted: Vec<_> = self.data.iter().enumerate().map(|(i, v)| (i, v)).collect();
    let mut merge_data = Vec::with_capacity(resorted.len());
    let mut deduplicate_map = Vec::with_capacity(self.index.len());
    resorted.sort_unstable_by(|a, b| sorter(a.1, b.1));

    let mut resort_map: Vec<_> = (0..self.data.len()).collect();
    resorted
      .iter()
      .enumerate()
      .for_each(|(i, v)| resort_map[v.0] = i);

    if self.data.len() >= 2 {
      merge_data.push(*resorted[0].1);
      deduplicate_map.push(0);

      resorted.windows(2).for_each(|v| {
        if !merger(&v[0].1, &v[1].1) {
          merge_data.push(*v[1].1);
        }
        deduplicate_map.push(merge_data.len() - 1);
      });
    }

    let new_index = self
      .index
      .iter()
      .map(|i| {
        let k = (*i).into_usize();
        let after_sort = resort_map[k];
        I::from_usize(deduplicate_map[after_sort])
      })
      .collect();

    IndexedMesh::new(merge_data, new_index)
  }
}

impl<I, V, T, U> IndexedMesh<I, V, T, U>
where
  I: IndexType,
  T: PrimitiveTopologyMeta<V>,
  U: MeshDataContainer<V>,
  V: Copy,
{
  pub fn expand_to_none_index_geometry(&self) -> NoneIndexedMesh<V, T, U> {
    NoneIndexedMesh::new(
      self
        .index
        .iter()
        .map(|i| self.data.as_ref()[(*i).into_usize()])
        .collect(),
    )
  }
}

impl<V, T> NoneIndexedMesh<V, T>
where
  V: HashAbleByConversion + Copy,
  T: IndexPrimitiveTopologyMeta<u16, V>,
  <T as PrimitiveTopologyMeta<V>>::Primitive: IndexedPrimitiveData<u16, V, Vec<V>, Vec<u16>>,
  // U: MeshDataContainer<V>, // ditto
{
  pub fn create_index_geometry(&self) -> IndexedMesh<u16, V, T> {
    let mut deduplicate_map = HashMap::<V::HashAble, usize>::new();
    let mut deduplicate_buffer = Vec::with_capacity(self.data.len());
    let index = self
      .data
      .iter()
      .map(|v| {
        let h = v.to_hashable();
        *deduplicate_map.entry(h).or_insert_with(|| {
          deduplicate_buffer.push(*v);
          deduplicate_buffer.len() - 1
        }) as u16
      })
      .collect();
    deduplicate_buffer.shrink_to_fit();
    IndexedMesh::new(deduplicate_buffer, index)
  }
}

impl<I, V, T, U> IndexedMesh<I, V, T, U>
where
  T: PrimitiveTopologyMeta<V>,
  U: MeshDataContainer<V>,
{
  pub fn create_point_cloud(&self) -> NoneIndexedMesh<V, PointList, U> {
    NoneIndexedMesh::new(self.data.clone())
  }
}
