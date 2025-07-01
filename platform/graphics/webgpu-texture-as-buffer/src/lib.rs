//! this crate demonstrate a criminal and beautiful hack to implement texture as read-only
//! storage buffer using the powerful `AbstractPtr` mechanism in our shader framework.
//! the reason to do so is to (prototype) support indirect rendering on gles-only platform
//! (should work with the other features like MIDC downgrade and storage buffer auto-merge).

use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

pub struct TextureAsReadonlyStorageBuffer<T> {
  phantom: PhantomData<T>,
  texture: GPUTypedTextureView<TextureDimension2, u32>,
  ty: ShaderValueSingleType,
}

impl<T> TextureAsReadonlyStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderSizedValueNodeType,
{
  pub fn new(init: StorageBufferInit<T>, gpu: &GPU) -> Self {
    // let buffer_init = init.into_buffer_init();

    // gpu.device.create_texture(desc)
    // todo, check u32 length is valid
    todo!()
  }

  pub fn build_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> ShaderReadonlyPtrOf<T> {
    let texture = bind_builder.bind_by(&self.texture);
    let ptr = U32HeapPtrWithType {
      ptr: U32HeapPtr {
        array: U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(Box::new(
          TextureAsU32Heap { texture },
        ))),
        offset: val(0),
      },
      ty: self.ty.clone(),
      bind_index: val(0),
      meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(
        StructLayoutTarget::Std430,
      ))),
    };

    let ptr = Box::new(TextureU32HeapPtrWithType { internal: ptr });
    T::create_readonly_view_from_raw_ptr(ptr)
  }

  pub fn bind_pass(&mut self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(&self.texture);
  }
}

/// wrapped to control access.
#[derive(Clone)]
pub struct TextureU32HeapPtrWithType {
  internal: U32HeapPtrWithType,
}

impl AbstractShaderPtr for TextureU32HeapPtrWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    self.internal.field_index(field_index)
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    self.internal.field_array_index(index)
  }

  fn array_length(&self) -> Node<u32> {
    self.internal.array_length()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    self.internal.load()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}

/// implement an "array", another way to do so is adding a new trait to abstract
/// the `ShaderReadonlyPtrOf<T>`.
#[derive(Clone)]
struct TextureAsU32Heap {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
}

impl AbstractShaderPtr for TextureAsU32Heap {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let width = self.texture.texture_dimension_2d(None).x();
    let index = index + val(1);
    let x = index % width;
    let y = index / width;
    Box::new(TextureAsU32HeapPosition {
      texture: self.texture,
      position: (x, y).into(),
    })
  }

  fn array_length(&self) -> Node<u32> {
    self.texture.load_texel(val(Vec2::zero()), val(0)).x()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}

#[derive(Clone)]
struct TextureAsU32HeapPosition {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  position: Node<Vec2<u32>>,
}

impl AbstractShaderPtr for TextureAsU32HeapPosition {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, _: Node<u32>) -> BoxedShaderPtr {
    unreachable!()
  }

  fn array_length(&self) -> Node<u32> {
    unreachable!()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    self.texture.load_texel(self.position, val(0)).x().handle()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}
