use hierarchy_reducer::HierarchyMonoidReducer;

use crate::*;

pub fn reduce_impl<OneKey: CKey, ManyKey, ManyValue>(
  many: impl DualQueryLike<Key = ManyKey, Value = ManyValue>,
  relation: impl DualQueryLike<Key = ManyKey, Value = OneKey>,
  states: &mut impl AbstractReducer<OneKey, ManyKey, ManyValue>,
) -> Arc<FastHashMap<OneKey, ValueChange<ManyValue>>> {
  let (many, many_delta) = many.view_delta();
  let (relation, relation_delta) = relation.view_delta();

  for (many_key, relation_change) in relation_delta.iter_key_value() {
    match relation_change {
      ValueChange::Delta(new_one, _) => {
        if let Some(many) = many.access(&many_key) {
          states.notify_insert_or_update(new_one.clone(), many_key, many);
        }
      }
      ValueChange::Remove(old_one) => {
        // this call may remove not exist one, but it's ok
        states.notify_remove(&old_one, &many_key);
      }
    }
  }

  for (many_key, many_change) in many_delta.iter_key_value() {
    match many_change {
      ValueChange::Delta(new_many, _) => {
        if let Some(one) = relation.access(&many_key) {
          states.notify_insert_or_update(one.clone(), many_key, new_many);
        }
      }
      ValueChange::Remove(_) => {
        // the relation's previous one has been removed above.
        if let Some(one) = relation.access(&many_key) {
          states.notify_remove(&one, &many_key);
        }
      }
    }
  }

  Arc::new(states.update())
}

pub trait AbstractReducer<KOne, K, T> {
  // remove none exist is allowed
  fn notify_remove(&mut self, one_key: &KOne, key: &K);
  fn notify_insert_or_update(&mut self, one_key: KOne, key: K, value: T);
  fn update(&mut self) -> FastHashMap<KOne, ValueChange<T>>;
}

pub struct HierarchyMonoidReducerGroup<KOne, K, T, F> {
  mapping: FastHashMap<KOne, HierarchyMonoidReducer<K, T>>,
  changed: FastHashSet<KOne>,
  reducer: F,
}

impl<KOne: CKey, K: CKey, T: CValue, F: Send + Sync> Query
  for LockReadGuardHolder<HierarchyMonoidReducerGroup<KOne, K, T, F>>
{
  type Key = KOne;
  type Value = T;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .mapping
      .iter()
      .map(|(k, v)| (k.clone(), v.current_value().unwrap().clone()))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self
      .mapping
      .get(key)
      .map(|v| v.current_value().unwrap().clone())
  }

  fn has_item_hint(&self) -> bool {
    self.mapping.len() > 0
  }
}

impl<KOne, K, T, F> HierarchyMonoidReducerGroup<KOne, K, T, F> {
  pub fn new(reducer: F) -> Self {
    Self {
      mapping: FastHashMap::default(),
      changed: FastHashSet::default(),
      reducer,
    }
  }
}

impl<KOne, K, T, F> AbstractReducer<KOne, K, T> for HierarchyMonoidReducerGroup<KOne, K, T, F>
where
  KOne: Hash + Eq + Clone,
  K: Hash + Eq + Clone,
  T: Clone + PartialEq,
  F: Fn(T, T) -> T,
{
  // remove none exist is allowed
  fn notify_remove(&mut self, one_key: &KOne, key: &K) {
    if let Some(reducer) = self.mapping.get_mut(one_key) {
      reducer.notify_remove(key);
      self.changed.insert(one_key.clone());
    }
  }

  fn notify_insert_or_update(&mut self, one_key: KOne, key: K, value: T) {
    self
      .mapping
      .entry(one_key.clone())
      .or_default()
      .notify_insert_or_update(key, value);
    self.changed.insert(one_key);
  }

  fn update(&mut self) -> FastHashMap<KOne, ValueChange<T>> {
    let mut changes = FastHashMap::default();
    let reducer_ref = &self.reducer;

    let mut empty_keys = Vec::new();
    for one_key in self.changed.drain() {
      let Some(reducer) = self.mapping.get_mut(&one_key) else {
        continue;
      };
      let old_v = reducer.current_value().cloned();
      let new_v = reducer.update(reducer_ref);

      match (old_v, new_v) {
        (Some(old), Some(new)) if old == new => {
          // value unchanged, no delta
        }
        (Some(old), Some(new)) => {
          changes.insert(one_key, ValueChange::Delta(new, Some(old)));
        }
        (None, Some(new)) => {
          changes.insert(one_key, ValueChange::Delta(new, None));
        }
        (Some(old), None) => {
          changes.insert(one_key.clone(), ValueChange::Remove(old));
          empty_keys.push(one_key);
        }
        (None, None) => {
          empty_keys.push(one_key);
        }
      }
    }

    for key in empty_keys {
      self.mapping.remove(&key);
    }

    changes
  }
}
