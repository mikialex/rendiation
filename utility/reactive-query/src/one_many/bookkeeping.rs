use storage::{LinkListPool, ListHandle};

use crate::*;

pub struct OneToManyRefHashBookKeeping<T, K, V> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<V, FastHashSet<K>>>>,
}

impl<T> ReactiveQuery for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: ReactiveQuery,
  T::Value: CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Compute = OneToManyRefHashBookKeeping<T::Compute, T::Key, T::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToManyRefHashBookKeeping {
      upstream: self.upstream.describe(cx),
      mapping: self.mapping.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
  }
}

impl<T> QueryCompute for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: QueryCompute,
  T::Value: CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Changes = T::Changes;
  type View =
    QueryAndMultiQuery<T::View, LockReadGuardHolder<FastHashMap<T::Value, FastHashSet<T::Key>>>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.resolve(cx);

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

    let v = QueryAndMultiQuery {
      query: r_view,
      multi_query: self.mapping.make_read_holder(),
    };

    (r, v)
  }
}
impl<T> AsyncQueryCompute for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: AsyncQueryCompute,
  T::Value: CKey,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapping = self.mapping.clone();
    let upstream = self.upstream.create_task(cx);
    cx.then_spawn(upstream, |upstream, cx| {
      OneToManyRefHashBookKeeping { upstream, mapping }.resolve(cx)
    })
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

impl<T> Query for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: Query,
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

impl<T> MultiQuery for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: Query,
  T::Key: CKey + LinearIdentification,
  T::Value: CKey + LinearIdentification,
{
  type Key = T::Value;
  type Value = T::Key;
  fn iter_keys(&self) -> impl Iterator<Item = T::Value> + '_ {
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

impl<T> ReactiveQuery for OneToManyRefDenseBookKeeping<T>
where
  T: ReactiveQuery,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Key = T::Key;
  type Value = T::Value;
  type Compute = OneToManyRefDenseBookKeeping<T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToManyRefDenseBookKeeping {
      upstream: self.upstream.describe(cx),
      mapping: self.mapping.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => {
        let mut mapping = self.mapping.write();
        mapping.mapping.shrink_to_fit();
        mapping.mapping_buffer.shrink_to_fit();
      }
    }
  }
}

impl<T> QueryCompute for OneToManyRefDenseBookKeeping<T>
where
  T: QueryCompute,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Changes = T::Changes;
  type View = OneToManyRefDenseBookKeepingCurrentView<T::View>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.resolve(cx);

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
}

impl<T> AsyncQueryCompute for OneToManyRefDenseBookKeeping<T>
where
  T: AsyncQueryCompute,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapping = self.mapping.clone();
    let upstream = self.upstream.create_task(cx);
    let f = cx.then_spawn(upstream, |upstream, cx| {
      OneToManyRefDenseBookKeeping { upstream, mapping }.resolve(cx)
    });

    avoid_huge_debug_symbols_by_boxing_future(f)
  }
}
