use crate::*;

pub struct DBWatchScope {
  pub change: DBLinearChangeWatchGroup,
  pub query: DBQueryChangeWatchGroup,
  pub query_set: DBQueryEntitySetWatchGroup,
}

impl DBWatchScope {
  pub fn new(db: &Database) -> Self {
    Self {
      change: DBLinearChangeWatchGroup::new(db),
      query: DBQueryChangeWatchGroup::new(db),
      query_set: DBQueryEntitySetWatchGroup::new(db),
    }
  }

  pub fn clear_changes(&mut self) {
    self.change.clear_changes();
    self.query.clear_changes();
    self.query_set.clear_changes();
  }
}

pub(crate) struct DBChangeWatchGroup<K> {
  pub producers: FastHashMap<K, Box<dyn Any + Send + Sync>>,
  pub consumers: FastHashMap<K, FastHashSet<u32>>,
  pub current_results: FastHashMap<K, Box<dyn Any + Send + Sync>>,
  pub next_consumer_id: u32,
  pub db: Database,
}

impl<K: Eq + Hash> DBChangeWatchGroup<K> {
  pub fn new(db: &Database) -> Self {
    Self {
      producers: Default::default(),
      consumers: Default::default(),
      current_results: Default::default(),
      next_consumer_id: 0,
      db: db.clone(),
    }
  }

  pub fn clear_changes(&mut self) {
    self.current_results.clear();
  }

  pub fn allocate_next_consumer_id(&mut self) -> u32 {
    self.next_consumer_id += 1;
    self.next_consumer_id
  }

  pub fn notify_consumer_dropped(&mut self, key: K, consumer_id: u32) {
    let consumers = self.consumers.get_mut(&key).unwrap();
    let removed = consumers.remove(&consumer_id);
    assert!(removed);
    if consumers.is_empty() {
      self.producers.remove(&key); // stop the watch
      self.consumers.remove(&key);
    }
  }
}
