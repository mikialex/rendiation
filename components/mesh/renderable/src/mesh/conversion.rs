//! The conversion method between different mesh types

// todo for convert between different mesh type

// downgrade:
// mesh -> line , wireframe? edge?
// line -> point

// indexed -> noneIndexed expand?
// noneIndexed -> indexed indexed?

use super::{
  AbstractIndexMesh, AbstractMesh, CollectionSize, HashAbleByConversion, IndexGet, IndexType,
  IndexedMesh, IndexedPrimitiveData, LineList, NoneIndexedMesh, PointList, PrimitiveTopologyMeta,
  TryFromIterator,
};
use rendiation_algebra::{InnerProductSpace, Vec3};
use rendiation_geometry::{LineSegment, Triangle};
use std::{
  cmp::Ordering,
  collections::{HashMap, HashSet},
  iter::FromIterator,
  ops::Deref,
};

impl<T, U, IU> IndexedMesh<T, U, IU>
where
  Self: AbstractIndexMesh<IndexPrimitive = Triangle<IU::Output>>,
  U: Clone,
  IU: IndexGet,
{
  pub fn create_wireframe<RIU>(&self) -> IndexedMesh<LineList, U, RIU>
  where
    RIU: FromIterator<IU::Output>,
  {
    let mut deduplicate_set = HashSet::<LineSegment<IU::Output>>::new();
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
    IndexedMesh::new(self.data.clone(), new_index)
  }
}

impl<T, U, IU> IndexedMesh<T, U, IU>
where
  Self: AbstractIndexMesh<IndexPrimitive = Triangle<IU::Output>, Primitive = Triangle<U::Output>>,
  U: IndexGet + FromIterator<U::Output>,
  IU: IndexGet,
  // I: IndexType,
  // V: Copy + Deref<Target = Vec3<f32>>,
{
  /// maybe you should merge vertex before create edge
  /// non manifold mesh may affect result
  pub fn create_edge(&self, edge_threshold_angle: f32) -> NoneIndexedMesh<LineList, U> {
    // Map: edge id => (edge face idA, edge face idB(optional));
    let mut edges = HashMap::<LineSegment<IU::Output>, (usize, Option<usize>)>::new();
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
      .map(|i| self.data.index_get(i.into_usize()).unwrap())
      .collect();
    NoneIndexedMesh::new(data)
  }
}

impl<T, U, IU> IndexedMesh<T, U, IU>
where
  IU: IndexGet + TryFromIterator<usize> + CollectionSize,
  for<'a> &'a IU: IntoIterator<Item = IU::Output>,
  U: IndexGet,
  Self: AbstractIndexMesh,
{
  pub fn merge_vertex_by_sorting(
    &self,
    mut sorter: impl FnMut(&U::Output, &U::Output) -> Ordering,
    mut merger: impl FnMut(&U::Output, &U::Output) -> bool,
  ) -> Result<IndexedMesh<T, Vec<U::Output>, IU>, IU::Error> {
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
        if !merger(v[0].1, v[1].1) {
          merge_data.push(*v[1].1);
        }
        deduplicate_map.push(merge_data.len() - 1);
      });
    }

    let index = &self.index;
    let new_index = IU::try_from_iter(index.into_iter().map(|i| resort_map[i.into_usize()]))?;

    Ok(IndexedMesh::new(merge_data, new_index))
  }
}

impl<T, U, IU> IndexedMesh<T, U, IU>
where
  IU: IndexGet,
  U: IndexGet + FromIterator<U::Output>,
  for<'a> &'a IU: IntoIterator<Item = IU::Output>,
{
  pub fn expand_to_none_index_geometry(&self) -> NoneIndexedMesh<T, U> {
    let index = &self.index;
    NoneIndexedMesh::new(
      index
        .into_iter()
        .map(|i| self.data.index_get((i).into_usize()).unwrap())
        .collect(),
    )
  }
}

impl<V, T> NoneIndexedMesh<V, T>
where
  V: HashAbleByConversion + Copy,
  T: PrimitiveTopologyMeta<V>,
  <T as PrimitiveTopologyMeta<V>>::Primitive: IndexedPrimitiveData<u16, V, Vec<V>, Vec<u16>>,
{
  pub fn create_index_geometry<I, IU>(&self) -> Result<IndexedMesh<I, V, T, Vec<V>, IU>, IU::Error>
  where
    IU: TryFromIterator<usize>,
  {
    let mut deduplicate_map = HashMap::<V::HashAble, usize>::new();
    let mut deduplicate_buffer = Vec::with_capacity(self.data.len());
    let index = IU::try_from_iter(self.data.iter().map(|v| {
      let h = v.to_hashable();
      *deduplicate_map.entry(h).or_insert_with(|| {
        deduplicate_buffer.push(*v);
        deduplicate_buffer.len() - 1
      })
    }))?;
    deduplicate_buffer.shrink_to_fit();
    Ok(IndexedMesh::new(deduplicate_buffer, index))
  }
}

impl<T, U, IU> IndexedMesh<T, U, IU>
where
  Self: AbstractIndexMesh, // we add this bound for better semantic constraint
  U: Clone,
{
  pub fn create_point_cloud(&self) -> NoneIndexedMesh<PointList, U> {
    NoneIndexedMesh::new(self.data.clone())
  }
}
