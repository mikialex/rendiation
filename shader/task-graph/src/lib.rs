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

pub trait ShaderAbstractLoadStore<T> {
  fn abstract_load(&self) -> T;
  fn abstract_store(&self, payload: T);
}
pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLoadStore<T>>;

impl<T> ShaderAbstractLoadStore<Node<T>> for LocalVarNode<T> {
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}
