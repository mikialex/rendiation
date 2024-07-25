use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Weak;

use fast_hash_collection::*;
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod runtime;
pub use runtime::*;

mod future;
pub use future::*;

/// abstract left value in shader
pub trait ShaderAbstractLoadStore {
  /// Value must a right value in shader
  type Value;
  fn abstract_load(&self) -> Self::Value;
  fn abstract_store(&self, payload: Self::Value);
}
pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLoadStore<Value = T>>;

impl<T> ShaderAbstractLoadStore for LocalVarNode<T> {
  type Value = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}
