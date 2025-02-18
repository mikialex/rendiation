use dyn_clone::DynClone;

use crate::*;

/// this trait is to abstract the interface of different shader ptr types, for example
/// StorageNode, UniformNode, or **user defined ptr-like object**
///
/// the implementation should be able to implement Clone because every shader node can be cloned
/// so any shader runtime-like object should be clonable as well.
///
/// as this trait is untyped, the method may not valid to implement for some type, for example
/// it's impossible to array index to a none-array object. when this case happens, the method should
/// do what it could do and raise the proper validation error in later process.
pub trait AbstractShaderPtr: DynClone {
  fn field_index(&self, index: u32) -> Box<dyn AbstractShaderPtr>;
  fn field_array_index(&self, index: Node<u32>) -> Box<dyn AbstractShaderPtr>;
  fn array_length(&self) -> Node<u32>;
  fn load(&self) -> ShaderNodeRawHandle;
  fn store(&self, value: ShaderNodeRawHandle);
}
impl Clone for Box<dyn AbstractShaderPtr> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl AbstractShaderPtr for ShaderNodeRawHandle {
  fn field_index(&self, index: u32) -> Box<dyn AbstractShaderPtr> {
    todo!()
  }

  fn field_array_index(&self, index: Node<u32>) -> Box<dyn AbstractShaderPtr> {
    todo!()
  }

  fn array_length(&self) -> Node<u32> {
    todo!()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    todo!()
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    todo!()
  }
}

/// this trait is to mapping the `T` to it's typed shader access object. the access object
/// has type api to constraint valid access.
pub trait ShaderNodeAbstractAccessSource {
  type Access;
  fn create_accessor_from_raw_ptr<Ptr: AbstractShaderPtr>(ptr: Ptr) -> Self;
}

struct ArrayAccess<T, Ptr: AbstractShaderPtr> {
  phantom: PhantomData<T>,
  access: Ptr,
}

impl<T: ShaderNodeAbstractAccessSource, Ptr: AbstractShaderPtr> ArrayAccess<T, Ptr> {
  pub fn index(&self, index: Node<u32>) -> T::Access {
    let item = self.access.field_array_index(index);
    // T::create_accessor_from_raw_ptr(item)
    todo!()
  }
}
