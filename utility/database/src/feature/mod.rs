mod rev_ref;
mod watch;

pub use rev_ref::*;
pub use watch::*;

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
