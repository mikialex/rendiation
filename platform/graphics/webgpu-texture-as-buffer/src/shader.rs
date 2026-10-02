use crate::*;

/// the array length stored in the header, only Some if the content type is a runtime sized array
pub(crate) fn runtime_array_length(ty_desc: &MaybeUnsizedValueType, byte_size: u64) -> Option<u32> {
  if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = ty_desc {
    let stride = array_stride_of_element(ty, StructLayoutTarget::Std430) as u64;
    Some((byte_size / stride) as u32)
  } else {
    None
  }
}

pub(crate) fn bind_heap_texture_shader(
  bind_builder: &mut ShaderBindGroupBuilder,
  texture: &GPUTypedTextureView<TextureDimension2, u32>,
  ty_desc: &MaybeUnsizedValueType,
) -> BoxedShaderPtr {
  let texture = bind_builder.bind_by(texture);
  let heap = TextureAsU32Heap {
    texture,
    width: texture.texture_dimension_2d(None).x(),
    length_from_header: false,
  };

  let runtime_array_ty =
    if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = ty_desc {
      Some(ty)
    } else {
      None
    };

  // the u32 array is the heap itself, the typed wrapper is not required. this also avoids the
  // nested u32 heap access when the combined buffer is built on top of this.
  if runtime_array_ty.is_some_and(|ty| **ty == u32::sized_ty()) {
    return Box::new(TextureAsU32Heap {
      length_from_header: true,
      ..heap
    });
  }

  let array_length = runtime_array_ty.map(|_| heap.header());

  let mut meta = ShaderU32StructMetaData::new(StructLayoutTarget::Std430);
  meta.register_ty(ty_desc);

  let ptr = U32HeapPtr {
    array: U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(Box::new(heap))),
    offset: val(0),
  };

  Box::new(U32HeapPtrWithType {
    ptr,
    ty: ty_desc.clone().into_shader_single_ty(),
    array_length,
    meta: Arc::new(RwLock::new(meta)),
  })
}

/// The readonly `[u32]` view of the texture, the index skips the header texel.
#[derive(Clone)]
struct TextureAsU32Heap {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  /// cache the texture dimension call result
  width: Node<u32>,
  /// if true, the array length is read from the header, otherwise it is the u32 capacity. the
  /// former is used when the content type is `[u32]`, the latter is used when the heap is wrapped by
  /// the typed u32 heap ptr, which compares the u32 offset with the array length when the bound
  /// check is enabled.
  length_from_header: bool,
}

impl TextureAsU32Heap {
  fn header(&self) -> Node<u32> {
    self.texture.load_texel(val(Vec2::zero()), 0).x()
  }
}

impl AbstractShaderPtr for TextureAsU32Heap {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let index = index + val(1);
    let x = index % self.width;
    let y = index / self.width;
    Box::new(TextureAsU32HeapPosition {
      texture: self.texture,
      position: (x, y).into(),
    })
  }

  fn array_length(&self) -> Node<u32> {
    if self.length_from_header {
      self.header()
    } else {
      let height = self.texture.texture_dimension_2d(None).y();
      self.width * height - val(1)
    }
  }

  fn load(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!("texture as storage buffer is readonly")
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
    self.texture.load_texel(self.position, 0).x().handle()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!("texture as storage buffer is readonly")
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!("texture as storage buffer does not support atomic")
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}
