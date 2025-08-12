mod delta;
pub use delta::*;

mod previous_view;
pub use previous_view::*;

mod fanout;

mod filter;
pub use filter::*;

mod mutate_target;
pub use mutate_target::*;

mod map;
pub use map::*;

use crate::*;

#[derive(Clone)]
pub struct DualQuery<T, U> {
  pub view: T,
  pub delta: U,
}

pub struct TriQuery<T, U, V> {
  pub base: DualQuery<T, U>,
  pub rev_many_view: V,
}

pub trait DeltaQueryExt<V>: Query<Value = ValueChange<V>> {
  fn delta_map<V2, F>(self, mapper: F) -> MappedQuery<Self, ValueChangeMapper<F>>
  where
    F: Fn(&Self::Key, V) -> V2 + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    MappedQuery {
      base: self,
      mapper: ValueChangeMapper(mapper),
    }
  }

  fn delta_map_value<V2, F>(
    self,
    mapper: F,
  ) -> MappedValueQuery<Self, ValueChangeMapperValueOnly<F>>
  where
    F: Fn(V) -> V2 + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    MappedValueQuery {
      base: self,
      mapper: ValueChangeMapperValueOnly(mapper),
    }
  }

  fn delta_filter_map<V2, F>(self, mapper: F) -> FilterMapQueryChange<Self, F>
  where
    F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    FilterMapQueryChange { base: self, mapper }
  }
}
impl<V, T: Query<Value = ValueChange<V>>> DeltaQueryExt<V> for T {}
