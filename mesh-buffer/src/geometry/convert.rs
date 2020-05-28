// todo for convert between different geometry type

// downgrade:
// mesh -> line , wireframe? edge?
// line -> point

// indexed -> noneIndexed expand?
// noneIndexed -> indexed indexed?

use super::{IndexedGeometry, NoneIndexedGeometry, PositionedPoint, PrimitiveTopology};
use std::collections::HashMap;

impl<V: PositionedPoint + Copy, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn expand_to_none_index_geometry(&self) -> NoneIndexedGeometry<V, T> {
    NoneIndexedGeometry::new(self.index.iter().map(|i| self.data[*i as usize]).collect())
  }
}

impl<V: PositionedPoint + Copy, T: PrimitiveTopology<V>> NoneIndexedGeometry<V, T> {
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
