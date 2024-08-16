use std::any::Any;
use std::any::TypeId;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::Weak;

use fast_hash_collection::*;
use parking_lot::RwLock;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod runtime;
pub use runtime::*;

mod future;
pub use future::*;

mod dyn_ty_builder;
pub use dyn_ty_builder::*;

mod bump_allocator;
pub use bump_allocator::*;

mod test;

/// abstract left value in shader
pub trait ShaderAbstractLeftValue {
  /// Value must a pure right value in shader (nested pointer is not allowed)
  type RightValue;
  fn abstract_load(&self) -> Self::RightValue;
  fn abstract_store(&self, payload: Self::RightValue);
}
pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLeftValue<RightValue = T>>;

impl<T> ShaderAbstractLeftValue for LocalVarNode<T> {
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}

pub trait ShaderAbstractRightValue {
  type LocalLeftValue: ShaderAbstractLeftValue<RightValue = Self>;
  fn into_local_left_value(self) -> Self::LocalLeftValue;
}

impl<T: ShaderNodeType> ShaderAbstractRightValue for Node<T> {
  type LocalLeftValue = LocalVarNode<T>;

  fn into_local_left_value(self) -> Self::LocalLeftValue {
    self.make_local_var()
  }
}
