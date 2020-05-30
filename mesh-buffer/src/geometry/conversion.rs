// todo for convert between different geometry type

// downgrade:
// mesh -> line , wireframe? edge?
// line -> point

// indexed -> noneIndexed expand?
// noneIndexed -> indexed indexed?

use super::{
  HashAbleByConversion, IndexedGeometry, LineList, NoneIndexedGeometry, PointList,
  PrimitiveTopology,
};
use rendiation_math::Vec3;
use rendiation_math_entity::{Face3, Line3, PositionedPoint};
use std::{
  cmp::Ordering,
  collections::{HashMap, HashSet},
};

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V, Primitive = Face3<V>>>
  IndexedGeometry<V, T>
{
  pub fn create_wireframe(&self) -> IndexedGeometry<V, LineList> {
    let mut deduplicate_set = HashSet::<Line3<u16>>::new();
    self.primitive_iter().for_each(|(_, f)| {
      f.for_each_edge(|edge| {
        deduplicate_set.insert(edge.swap_if(|l| l.start < l.end));
      })
    });
    let new_index = deduplicate_set
      .iter()
      .flat_map(|l| l.iter_point())
      .collect();
    IndexedGeometry::<V, LineList>::new(self.data.clone(), new_index)
  }

  /// maybe you should merge vertex before create edge
  /// non manifold mesh may affect result
  pub fn create_edge(&self, edge_threshold_angle: f32) -> NoneIndexedGeometry<V, LineList> {
    // Map: edge id => (edge face idA, edge face idB(optional));
    let mut edges = HashMap::<Line3<u16>, (usize, Option<usize>)>::new();
    self
      .primitive_iter()
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
      .map(|(f, _)| f.face_normal_by_position())
      .collect::<Vec<Vec3<f32>>>();
    let threshold_dot = edge_threshold_angle.cos();
    let data = edges
      .iter()
      .filter(|(_, f)| f.1.is_none() || normals[f.0].dot(normals[f.1.unwrap()]) <= threshold_dot)
      .map(|(e, _)| e)
      .flat_map(|l| l.iter_point())
      .map(|i| self.data[i as usize])
      .collect();
    NoneIndexedGeometry::new(data)
  }
}

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn merge_vertex_by_sorting(
    &self,
    sorter: impl FnMut(&V, &V) -> Ordering,
    mut merger: impl FnMut(&V, &V) -> bool,
  ) -> IndexedGeometry<V, T> {
    let mut data = self.data.clone();
    let mut merge_data = Vec::with_capacity(data.len());
    let mut index_remapping = HashMap::new();
    data.sort_unstable_by(sorter);
    data.windows(2).enumerate().for_each(|(i, v)| {
      if merger(&v[0], &v[1]) {
        index_remapping.insert(i + 1, merge_data.len() - 1);
      } else {
        merge_data.push(v[1]);
      }
    });
    let new_index = self
      .index
      .iter()
      .map(|i| {
        let k = *i as usize;
        *index_remapping.get(&k).unwrap_or(&k) as u16
      })
      .collect();

    IndexedGeometry::new(merge_data, new_index)
  }
}

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn expand_to_none_index_geometry(&self) -> NoneIndexedGeometry<V, T> {
    NoneIndexedGeometry::new(self.index.iter().map(|i| self.data[*i as usize]).collect())
  }
}

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V>> NoneIndexedGeometry<V, T> {
  pub fn create_index_geometry<U>(&self) -> IndexedGeometry<V, T> {
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
    IndexedGeometry::new(deduplicate_buffer, index)
  }
}

impl<V: PositionedPoint, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn create_point_cloud(&self) -> NoneIndexedGeometry<V, PointList> {
    NoneIndexedGeometry::new(self.data.clone())
  }
}
