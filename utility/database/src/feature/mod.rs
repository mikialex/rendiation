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
pub use watch_group::*;
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

pub type RevRefContainer<K, V> = Arc<RwLock<FastHashMap<K, FastHashSet<V>>>>;
pub type RevRefContainerRead<K, V> = LockReadGuardHolder<FastHashMap<K, FastHashSet<V>>>;
pub type RevRefForeignKeyRead = RevRefContainerRead<RawEntityHandle, RawEntityHandle>;

/// we can also using composer to implement this, like [get_db_view_typed_foreign]
pub struct RevRefForeignKeyReadTyped<C> {
  pub internal: RevRefForeignKeyRead,
  pub phantom: PhantomData<C>,
}

impl<C> Clone for RevRefForeignKeyReadTyped<C> {
  fn clone(&self) -> Self {
    Self {
      internal: self.internal.clone(),
      phantom: self.phantom,
    }
  }
}

impl<C: ForeignKeySemantic> MultiQuery for RevRefForeignKeyReadTyped<C> {
  type Key = EntityHandle<C::ForeignEntity>;
  type Value = EntityHandle<C::Entity>;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .internal
      .iter_keys()
      .map(|k| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(k) })
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    self
      .internal
      .access_multi(&key.handle)
      .map(|iter| iter.map(|v| unsafe { EntityHandle::<C::Entity>::from_raw(v) }))
  }
}

pub type RevRefForeignTriQuery = TriQuery<
  BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  BoxedDynQuery<RawEntityHandle, ValueChange<RawEntityHandle>>,
  RevRefForeignKeyRead,
>;
