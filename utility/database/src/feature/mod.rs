mod rc;
mod rev_ref;
mod watch;

pub use rev_ref::*;
pub use watch::*;

use crate::*;

#[derive(Default)]
pub struct DataBaseFeatureGroup {
  features: FastHashMap<TypeId, Box<dyn DataBaseFeatureBox>>,
}

pub trait DataBaseFeatureBox: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}
impl<T: Any + Send + Sync> DataBaseFeatureBox for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl DataBaseFeatureGroup {
  pub fn register_feature(&mut self, feature: impl DataBaseFeatureBox) {
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
