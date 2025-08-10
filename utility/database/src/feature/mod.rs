mod label;
mod rev_ref;
mod serialization;
mod watch;
mod watch_group;
mod watch_linear;
mod watch_query;

pub use label::*;
pub use rev_ref::*;
pub use serialization::*;
pub use watch::*;
pub(crate) use watch_group::*;
pub use watch_linear::*;
pub use watch_query::*;

use crate::*;

#[derive(Default)]
pub struct DataBaseFeatureGroup {
  features: FastHashMap<TypeId, Box<dyn DataBaseFeature>>,
}

pub trait DataBaseFeature: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl DataBaseFeatureGroup {
  pub fn register_feature(&mut self, feature: impl DataBaseFeature) {
    self.features.insert(feature.type_id(), Box::new(feature));
  }

  pub fn get_feature<T: Clone + 'static>(&self) -> T {
    self
      .features
      .get(&TypeId::of::<T>())
      .unwrap()
      .as_ref()
      .as_any()
      .downcast_ref::<T>()
      .unwrap()
      .clone()
  }
}

#[derive(Default)]
pub struct DBForeignKeySharedRevRefs {
  pub task_id_mapping: FastHashMap<ComponentId, u32>,
}

pub type RevRefContainer<K, V> = Arc<RwLock<FastHashMap<K, FastHashSet<V>>>>;
pub type RevRefContainerRead<K, V> = LockReadGuardHolder<FastHashMap<K, FastHashSet<V>>>;
pub type RevRefForeignKey = RevRefContainerRead<RawEntityHandle, RawEntityHandle>;
