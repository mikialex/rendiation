use crate::*;

pub type MeshInner<T> = Identity<T>;

pub struct MeshCell<T> {
  pub inner: Arc<RwLock<MeshInner<T>>>,
}

impl<T> MeshCell<T> {
  pub fn new(mesh: T) -> Self {
    let mesh = MeshInner::new(mesh);
    Self {
      inner: Arc::new(RwLock::new(mesh)),
    }
  }
}

impl<T> Clone for MeshCell<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
