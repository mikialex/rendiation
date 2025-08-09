use crate::*;

pub(crate) struct DBChangeWatchGroup {
  pub producers: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  pub consumers: FastHashMap<ComponentId, FastHashSet<u32>>,
  pub current_results: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  pub next_consumer_id: u32,
  pub db: Database,
}

impl DBChangeWatchGroup {
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

  pub fn notify_consumer_dropped(&mut self, component_id: ComponentId, consumer_id: u32) {
    let consumers = self.consumers.get_mut(&component_id).unwrap();
    let removed = consumers.remove(&consumer_id);
    assert!(removed);
    if consumers.is_empty() {
      self.producers.remove(&component_id); // stop the watch
      self.consumers.remove(&component_id);
    }
  }
}
