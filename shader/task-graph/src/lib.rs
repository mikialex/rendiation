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
pub trait ShaderAbstractLeftValue {
  /// Value must a right value in shader
  type RightValue;
  fn abstract_load(&self) -> Self::RightValue;
  fn abstract_store(&self, payload: Self::RightValue);
}
pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLeftValue<RightValue = T>>;

pub trait ShaderAbstractRightValue {
  type LocalLeftValue: ShaderAbstractLeftValue<RightValue = Self>;
  fn into_local_left_value(self) -> Self::LocalLeftValue;
}

impl<T> ShaderAbstractLeftValue for LocalVarNode<T> {
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}
