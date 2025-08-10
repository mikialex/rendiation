use crate::*;

pub fn bookkeeping_hash_relation<K: CKey, V: CKey>(
  mapping: &mut FastHashMap<V, FastHashSet<K>>,
  changes: impl Query<Key = K, Value = ValueChange<V>>,
) {
  for (many, change) in changes.iter_key_value() {
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

#[derive(Default, Clone)]
pub struct DenseIndexMapping {
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
}

impl DenseIndexMapping {
  pub fn shrink_to_fit(&mut self) {
    self.mapping.shrink_to_fit();
    self.mapping.shrink_to_fit();
  }
}

impl MultiQuery for DenseIndexMapping {
  type Key = u32;
  type Value = u32;
  fn iter_keys(&self) -> impl Iterator<Item = u32> + '_ {
    self
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| (!list.is_empty()).then_some(i as u32))
  }

  fn access_multi(&self, o: &u32) -> Option<impl Iterator<Item = u32> + '_> {
    self
      .mapping
      .get(o.alloc_index() as usize)
      .map(|list| self.mapping_buffer.iter_list(list).map(|(v, _)| *v))
  }
}

pub fn bookkeeping_dense_index_relation<
  K: CKey + LinearIdentification,
  V: CKey + LinearIdentification,
>(
  mapping: &mut DenseIndexMapping,
  changes: impl Query<Key = K, Value = ValueChange<V>>,
) {
  for (many, change) in changes.iter_key_value() {
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
