use std::hash::Hash;

use crate::*;

#[derive(Clone)]
pub enum IndependentItemContainerDelta<K, T: IncrementalBase> {
  Remove(K),
  Insert(K, T),
  Mutate(K, DeltaOf<T>),
}

#[derive(Clone, Debug)]
pub enum ContainerRefRetainDelta<K, T> {
  Remove(K),
  Insert(K, T),
}

#[derive(Clone, Debug)]
pub enum ContainerRefRetainContentDelta<T> {
  Remove(T),
  Insert(T),
}

impl<T: IncrementalBase> From<ArenaDelta<T>>
  for IndependentItemContainerDelta<arena::Handle<T>, T>
{
  fn from(value: ArenaDelta<T>) -> Self {
    match value {
      ArenaDelta::Mutate((delta, handle)) => IndependentItemContainerDelta::Mutate(handle, delta),
      ArenaDelta::Insert((data, handle)) => IndependentItemContainerDelta::Insert(handle, data),
      ArenaDelta::Remove(handle) => IndependentItemContainerDelta::Remove(handle),
    }
  }
}

use arena::ArenaDelta;
use futures::*;
pub trait IncrementalStreamTransform {
  fn transform_ref_retained_to_ref_retained_content_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainContentDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static;

  fn transform_delta_to_ref_retained_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainDelta<K, T>>
  where
    Self: Stream<Item = IndependentItemContainerDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static + IncrementalBase<Delta = T>;
}

impl<X> IncrementalStreamTransform for X {
  fn transform_ref_retained_to_ref_retained_content_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainContentDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static,
  {
    let mut cache: FastHashMap<K, T> = Default::default();
    self.map(move |v| match v {
      ContainerRefRetainDelta::Remove(key) => {
        let value = cache
          .remove(&key)
          .expect("failed to retrieve source value in retained content transformation");
        ContainerRefRetainContentDelta::Remove(value)
      }
      ContainerRefRetainDelta::Insert(k, v) => {
        cache.insert(k, v.clone());
        ContainerRefRetainContentDelta::Insert(v)
      }
    })
  }

  fn transform_delta_to_ref_retained_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainDelta<K, T>>
  where
    Self: Stream<Item = IndependentItemContainerDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static + IncrementalBase<Delta = T>,
  {
    let mut cache: FastHashSet<K> = Default::default();
    self
      .map(move |v| {
        // this one will always stay on stack
        let mut one_or_two = smallvec::SmallVec::<[_; 2]>::default();
        match v {
          IndependentItemContainerDelta::Remove(key) => {
            assert!(cache.remove(&key));
            one_or_two.push(ContainerRefRetainDelta::Remove(key));
          }
          IndependentItemContainerDelta::Insert(key, value) => {
            assert!(cache.insert(key.clone()));
            one_or_two.push(ContainerRefRetainDelta::Insert(key, value));
          }
          IndependentItemContainerDelta::Mutate(key, value) => {
            if cache.remove(&key) {
              one_or_two.push(ContainerRefRetainDelta::Remove(key.clone()));
            }
            one_or_two.push(ContainerRefRetainDelta::Insert(key, value));
          }
        };
        futures::stream::iter(one_or_two)
      })
      .flatten()
  }
}

// pub trait LinearKey {
//   fn get_index(&self) -> usize;
// }

// impl<T> LinearKey for arena::Handle<T> {
//   fn get_index(&self) -> usize {
//     self.index()
//   }
// }
