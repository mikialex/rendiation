use storage::{LinkListPool, ListHandle};

use crate::*;

pub struct OneToManyRefHashBookKeeping<T: ReactiveCollection> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<T::Value, FastHashSet<T::Key>>>>,
}

#[derive(Clone)]
pub struct OneToManyRefHashBookKeepingCurrentView<T, M: CKey, O: CKey> {
  upstream: T,
  mapping: LockReadGuardHolder<FastHashMap<O, FastHashSet<M>>>,
}

impl<T, O, M> VirtualCollection<M, O> for OneToManyRefHashBookKeepingCurrentView<T, M, O>
where
  T: VirtualCollection<M, O>,
  M: CKey,
  O: CKey,
{
  fn access(&self, m: &M) -> Option<O> {
    self.upstream.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (M, O)> + '_ {
    self.upstream.iter_key_value()
  }
}

impl<T, O, M> VirtualMultiCollection<O, M> for OneToManyRefHashBookKeepingCurrentView<T, M, O>
where
  T: VirtualCollection<M, O>,
  M: CKey,
  O: CKey,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    self.mapping.keys().cloned()
  }

  fn access_multi(&self, o: &O) -> Option<impl Iterator<Item = M> + '_> {
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

  type Changes = impl VirtualCollection<T::Key, ValueChange<T::Value>>;
  type View = impl VirtualMultiCollection<T::Value, T::Key> + VirtualCollection<T::Key, T::Value>;

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

impl<T, O, M> VirtualCollection<M, O> for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: VirtualCollection<M, O>,
  M: CKey,
  O: CKey,
{
  fn access(&self, m: &M) -> Option<O> {
    self.upstream.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (M, O)> + '_ {
    self.upstream.iter_key_value()
  }
}

impl<T, O, M> VirtualMultiCollection<O, M> for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: VirtualCollection<M, O>,
  M: CKey + LinearIdentification,
  O: CKey + LinearIdentification,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    self
      .mapping
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| (!list.is_empty()).then_some(O::from_alloc_index(i as u32)))
  }

  fn access_multi(&self, o: &O) -> Option<impl Iterator<Item = M> + '_> {
    self
      .mapping
      .mapping
      .get(o.alloc_index() as usize)
      .map(|list| {
        self
          .mapping
          .mapping_buffer
          .iter_list(list)
          .map(|(v, _)| M::from_alloc_index(*v))
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
  type Changes = impl VirtualCollection<T::Key, ValueChange<T::Value>>;
  type View = impl VirtualMultiCollection<T::Value, T::Key> + VirtualCollection<T::Key, T::Value>;

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
