use std::{future::Future, sync::Arc};

use facet::*;
use fast_hash_collection::FastHashMap;
use serde::*;

#[repr(C)]
#[derive(Facet, Serialize, Deserialize, Clone)]
pub enum MaybeUriData<T> {
  Uri(Arc<String>),
  Living(T),
}

impl<T> MaybeUriData<T> {
  pub fn into_living(self) -> Option<T> {
    match self {
      MaybeUriData::Uri(_) => None,
      MaybeUriData::Living(v) => Some(v),
    }
  }
  pub fn as_living(&self) -> Option<&T> {
    match self {
      MaybeUriData::Uri(_) => None,
      MaybeUriData::Living(v) => Some(v),
    }
  }
}

impl<T: Default> Default for MaybeUriData<T> {
  fn default() -> Self {
    MaybeUriData::Living(T::default())
  }
}

pub trait UriDataSource<T>: Send + Sync {
  fn create_for_direct_data(&mut self, data: T) -> &str;
  fn request_uri_data_load(
    &mut self,
    uri: &str,
  ) -> impl Future<Output = Option<T>> + Unpin + Send + Sync + 'static;
  fn clear_init_direct_data(&mut self);
}

pub trait UriDataSourceDyn<T>: Send + Sync {
  fn create_for_direct_data(&mut self, data: T) -> &str;
  fn request_uri_data_load(
    &mut self,
    uri: &str,
  ) -> Box<dyn Future<Output = Option<T>> + Unpin + Send + Sync>;
  fn clear_init_direct_data(&mut self);
}

impl<X: UriDataSource<T>, T: 'static> UriDataSourceDyn<T> for X {
  fn create_for_direct_data(&mut self, data: T) -> &str {
    self.create_for_direct_data(data)
  }

  fn request_uri_data_load(
    &mut self,
    uri: &str,
  ) -> Box<dyn Future<Output = Option<T>> + Unpin + Send + Sync> {
    Box::new(self.request_uri_data_load(uri))
  }
  fn clear_init_direct_data(&mut self) {
    self.clear_init_direct_data()
  }
}

pub struct InMemoryUriDataSource<T> {
  source_id: u64,
  next_id: u64,
  data: FastHashMap<String, T>,
}

impl<T> InMemoryUriDataSource<T> {
  pub fn new(source_id: u64) -> Self {
    Self {
      source_id,
      next_id: Default::default(),
      data: Default::default(),
    }
  }
}

impl<T: 'static + Send + Sync + Clone> UriDataSource<T> for InMemoryUriDataSource<T> {
  fn create_for_direct_data(&mut self, data: T) -> &str {
    let key = format!("{}:{}", self.source_id, self.next_id);
    self.next_id += 1;
    self.data.insert(key.clone(), data);
    // is returning str really a good idea? because string clone is unavoidable
    self.data.get_key_value(&key).unwrap().0.as_str()
  }

  fn request_uri_data_load(
    &mut self,
    uri: &str,
  ) -> impl Future<Output = Option<T>> + Unpin + Send + Sync + 'static {
    let source = self.data.get(uri).cloned();
    std::future::ready(source)
  }

  fn clear_init_direct_data(&mut self) {
    // do nothing
  }
}
