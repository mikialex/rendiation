use storage::{LinkListPool, ListHandle};

use crate::*;

pub struct OneToManyRefHashBookKeeping<T: ReactiveCollection> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<T::Value, FastHashSet<T::Key>>>>,
}

#[derive(Clone)]
pub struct OneToManyRefHashBookKeepingCurrentView<T: VirtualCollection> {
  upstream: T,
  mapping: LockReadGuardHolder<FastHashMap<T::Value, FastHashSet<T::Key>>>,
}

impl<T> VirtualCollection for OneToManyRefHashBookKeepingCurrentView<T>
where
  T: VirtualCollection,
{
  type Key = T::Key;
  type Value = T::Value;
  fn access(&self, m: &T::Key) -> Option<T::Value> {
    self.upstream.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.upstream.iter_key_value()
  }
}

impl<T> VirtualMultiCollection for OneToManyRefHashBookKeepingCurrentView<T>
where
  T: VirtualCollection<Value: CKey>,
{
  type Key = T::Value;
  type Value = T::Key;
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = T::Value> + '_ {
    self.mapping.keys().cloned()
  }

  fn access_multi(&self, o: &T::Value) -> Option<impl Iterator<Item = T::Key> + '_> {
    self.mapping.get(o).map(|set| set.iter().cloned())
  }
}

impl<T> ReactiveCollection for OneToManyRefHashBookKeeping<T>
where
  T: ReactiveCollection,
  T::Value: CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Changes = impl VirtualCollection<Key = T::Key, Value = ValueChange<T::Value>>;
  type View = impl VirtualMultiCollection<Key = T::Value, Value = T::Key>
    + VirtualCollection<Key = T::Key, Value = T::Value>;

  #[tracing::instrument(skip_all, name = "OneToManyRefHashBookKeeping")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.poll_changes(cx);

    {
      let mut mapping = self.mapping.write();

      for (many, change) in r.iter_key_value() {
        let new_one = change.new_value();

        let old_refed_one = change.old_value();
        // remove possible old relations
        if let Some(old_refed_one) = old_refed_one {
          let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
          previous_one_refed_many.remove(&many);
          if previous_one_refed_many.is_empty() {
            mapping.remove(old_refed_one);
          }
        }

        // setup new relations
        if let Some(new_one) = new_one {
          let new_one_refed_many = mapping.entry(new_one.clone()).or_default();
          new_one_refed_many.insert(many.clone());
        }
      }
    }

    let v = OneToManyRefHashBookKeepingCurrentView {
      upstream: r_view,
      mapping: self.mapping.make_read_holder(),
    };

    (r, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
  }
}

pub struct OneToManyRefDenseBookKeeping<T> {
  pub upstream: T,
  pub mapping: Arc<RwLock<Mapping>>,
}

#[derive(Default)]
pub struct Mapping {
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
}

#[derive(Clone)]
pub struct OneToManyRefDenseBookKeepingCurrentView<T> {
  upstream: T,
  mapping: LockReadGuardHolder<Mapping>,
}

impl<T> VirtualCollection for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: VirtualCollection,
{
  type Key = T::Key;
  type Value = T::Value;
  fn access(&self, m: &T::Key) -> Option<T::Value> {
    self.upstream.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.upstream.iter_key_value()
  }
}

impl<T> VirtualMultiCollection for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: VirtualCollection,
  T::Key: CKey + LinearIdentification,
  T::Value: CKey + LinearIdentification,
{
  type Key = T::Value;
  type Value = T::Key;
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = T::Value> + '_ {
    self
      .mapping
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| (!list.is_empty()).then_some(T::Value::from_alloc_index(i as u32)))
  }

  fn access_multi(&self, o: &T::Value) -> Option<impl Iterator<Item = T::Key> + '_> {
    self
      .mapping
      .mapping
      .get(o.alloc_index() as usize)
      .map(|list| {
        self
          .mapping
          .mapping_buffer
          .iter_list(list)
          .map(|(v, _)| T::Key::from_alloc_index(*v))
      })
  }
}

impl<T> ReactiveCollection for OneToManyRefDenseBookKeeping<T>
where
  T: ReactiveCollection,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Key = T::Key;
  type Value = T::Value;
  type Changes = impl VirtualCollection<Key = T::Key, Value = ValueChange<T::Value>>;
  type View = impl VirtualMultiCollection<Key = T::Value, Value = T::Key>
    + VirtualCollection<Key = T::Key, Value = T::Value>;

  #[tracing::instrument(skip_all, name = "OneToManyRefDenseBookKeeping")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.poll_changes(cx);

    {
      let mut mapping = self.mapping.write();
      let mapping: &mut Mapping = &mut mapping;
      for (many, change) in r.iter_key_value() {
        let new_one = change.new_value();

        let old_refed_one = change.old_value();
        // remove possible old relations
        if let Some(old_refed_one) = old_refed_one {
          let previous_one_refed_many = mapping
            .mapping
            .get_mut(old_refed_one.alloc_index() as usize)
            .unwrap();

          //  this is O(n), should we care about it?
          mapping
            .mapping_buffer
            .visit_and_remove(previous_one_refed_many, |value, _| {
              let should_remove = *value == many.alloc_index();
              (should_remove, !should_remove)
            });
        }

        // setup new relations
        if let Some(new_one) = &new_one {
          let alloc_index = new_one.alloc_index() as usize;
          if alloc_index >= mapping.mapping.len() {
            mapping
              .mapping
              .resize(alloc_index + 1, ListHandle::default());
          }

          mapping.mapping_buffer.insert(
            &mut mapping.mapping[new_one.alloc_index() as usize],
            many.alloc_index(),
          );
        }
      }
    }

    let v = OneToManyRefDenseBookKeepingCurrentView {
      upstream: r_view,
      mapping: self.mapping.make_read_holder(),
    };

    (r, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => {
        let mut mapping = self.mapping.write();
        mapping.mapping.shrink_to_fit();
        mapping.mapping_buffer.shrink_to_fit();
      }
    }
  }
}
