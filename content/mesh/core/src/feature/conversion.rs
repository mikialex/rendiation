use crate::*;

/// from a triangle iter to create a line list iter, the line shared in different adjacent triangle
/// will be deduplicated
pub fn create_wireframe<T>(
  triangles: impl Iterator<Item = Triangle<T>>,
) -> impl Iterator<Item = LineSegment<T>>
where
  T: Copy + Clone + Eq + Ord + std::hash::Hash,
{
  let mut deduplicate_set = FastHashSet::<LineSegment<T>>::default();

  triangles.for_each(|face| {
    face.for_each_edge(|edge| {
      deduplicate_set.insert(edge.swap_if(|l| l.start < l.end));
    })
  });

  deduplicate_set.into_iter()
}

/// almost as same as create_wireframe, but if the edge is created depends on the adjacent face
/// angle threshold. Maybe you should merge vertex before create edge， non manifold mesh may affect
/// result
pub fn create_edges<T>(
  triangles: impl Iterator<Item = Triangle<T>>,
  edge_threshold_angle: f32,
) -> impl Iterator<Item = LineSegment<T>>
where
  T: Copy + Clone + Eq + Ord + std::hash::Hash + Positioned<Position = Vec3<f32>>,
{
  // todo, estimate capacity or use iter collect

  // Map: edge id => (edge face idA, edge face idB(optional));
  let mut edges = FastHashMap::<LineSegment<T>, (usize, Option<usize>)>::default();
  let mut normals = Vec::default();
  triangles.enumerate().for_each(|(face_id, face)| {
    normals.push(face.face_normal());
    face.for_each_edge(|edge| {
      edges
        .entry(edge.swap_if(|l| l.start < l.end))
        .and_modify(|e| e.1 = Some(face_id)) // if we find the e.1 is some already, the mesh is non manifold
        .or_insert_with(|| (face_id, None));
    })
  });

  let threshold_dot = edge_threshold_angle.cos();

  edges
    .into_iter()
    .filter(move |(_, f)| f.1.is_none() || normals[f.0].dot(normals[f.1.unwrap()]) <= threshold_dot)
    .map(|(edge, _)| edge)
}

// todo improve
impl<T, U, IU> IndexedMesh<T, U, IU>
where
  IU: IndexContainer + TryFromIterator<u32>,
  IU: IntoIterator<Item = IU::Output> + Clone,
  for<'a> &'a U: IntoIterator<Item = &'a U::Output>,
  U: VertexContainer,
  IU::Output: IndexType,
  U::Output: Copy,
{
  pub fn merge_vertex_by_sorting(
    &self,
    mut sorter: impl FnMut(&U::Output, &U::Output) -> std::cmp::Ordering,
    mut merger: impl FnMut(&U::Output, &U::Output) -> bool,
  ) -> Result<IndexedMesh<T, Vec<U::Output>, IU>, IU::Error> {
    let data = &self.vertex;
    let mut resorted: Vec<_> = data.into_iter().enumerate().collect();
    let mut merge_data = Vec::with_capacity(resorted.len());
    let mut deduplicate_map = Vec::with_capacity(self.index.len());
    resorted.sort_unstable_by(|a, b| sorter(a.1, b.1));

    let mut resort_map: Vec<_> = (0..self.vertex.len()).collect();
    resorted
      .iter()
      .enumerate()
      .for_each(|(i, v)| resort_map[v.0] = i);

    if self.vertex.len() >= 2 {
      merge_data.push(*resorted[0].1);
      deduplicate_map.push(0);

      resorted.windows(2).for_each(|v| {
        if !merger(v[0].1, v[1].1) {
          merge_data.push(*v[1].1);
        }
        deduplicate_map.push(merge_data.len() - 1);
      });
    }

    let new_index = IU::try_from_iter(
      self
        .index
        .clone()
        .into_iter()
        .map(|i| resort_map[i.into_usize()] as u32),
    )?;

    Ok(IndexedMesh::new(merge_data, new_index))
  }
}
