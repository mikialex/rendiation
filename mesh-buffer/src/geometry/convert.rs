// todo for convert between different geometry type

// downgrade:
// mesh -> line , wireframe? edge?
// line -> point

// indexed -> noneIndexed expand?
// noneIndexed -> indexed indexed?

use super::{
  HashAbleByConversion, IndexedGeometry, LineList, NoneIndexedGeometry, PositionedPoint,
  PrimitiveTopology,
};
use rendiation_math_entity::{Face3, Line3};
use std::collections::{HashMap, HashSet};

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V, Primitive = Face3<V>>>
  IndexedGeometry<V, T>
{
  pub fn create_wireframe(&self) -> IndexedGeometry<V, LineList> {
    let mut deduplicate_set = HashSet::<Line3<u16>>::new();
    self.primitive_iter().for_each(|(_, pi)| {
      pi.for_each_edge(|edge| {
        deduplicate_set.insert(edge.swap_if(|l| l.start < l.end));
      })
    });
    let new_index = deduplicate_set.iter().flat_map(|l| l.iter()).collect();
    IndexedGeometry::<V, LineList>::new(self.data.clone(), new_index)
  }
}

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn expand_to_none_index_geometry(&self) -> NoneIndexedGeometry<V, T> {
    NoneIndexedGeometry::new(self.index.iter().map(|i| self.data[*i as usize]).collect())
  }
}

impl<V: HashAbleByConversion + PositionedPoint, T: PrimitiveTopology<V>> NoneIndexedGeometry<V, T> {
  pub fn expand_to_none_index_geometry<U>(&self) -> IndexedGeometry<V, T> {
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
