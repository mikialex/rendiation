use crate::*;

/// the length of the runtime sized array, for the unsized struct it is the length of the last
/// field, return None for the sized type
pub(crate) fn runtime_array_length(ty_desc: &MaybeUnsizedValueType, byte_size: u64) -> Option<u32> {
  let layout = StructLayoutTarget::Std430;
  let (array_offset, ty) = match ty_desc {
    MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) => (0, ty),
    MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedStruct(ty)) => {
      let (array_offset, _) = ty.runtime_array_layout(layout);
      (array_offset as u64, &ty.last_dynamic_array_field.1)
    }
    MaybeUnsizedValueType::Sized(_) => return None,
  };
  let stride = array_stride_of_element(ty, layout) as u64;
  Some((byte_size.saturating_sub(array_offset) / stride) as u32)
}

/// `u32_per_texel` is 1 for the R32Uint texture, and 4 for the Rgba32Uint texture
pub(crate) fn bind_heap_texture_shader(
  bind_builder: &mut ShaderBindGroupBuilder,
  texture: &GPUTypedTextureView<TextureDimension2, u32>,
  ty_desc: &MaybeUnsizedValueType,
  u32_per_texel: u32,
) -> BoxedShaderPtr {
  assert!(u32_per_texel == 1 || u32_per_texel == 4);
  let texture = bind_builder.bind_by(texture);
  let heap = TextureAsU32Heap {
    texture,
    width: texture.texture_dimension_2d(None).x(),
    u32_per_texel,
    length_from_last_texel: false,
  };

  // the u32 array is the heap itself, the typed wrapper is not required. this also avoids the
  // nested u32 heap access when the combined buffer is built on top of this.
  if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = ty_desc
    && **ty == u32::sized_ty()
  {
    return Box::new(TextureAsU32Heap {
      length_from_last_texel: true,
      ..heap
    });
  }

  let is_unsized = matches!(ty_desc, MaybeUnsizedValueType::Unsized(_));
  let array_length = is_unsized.then(|| heap.last_texel());

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

/// The readonly `[u32]` view of the texture.
#[derive(Clone)]
struct TextureAsU32Heap {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  /// cache the texture dimension call result
  width: Node<u32>,
  u32_per_texel: u32,
  /// if true, the array length is read from the last texel, otherwise it is the u32 capacity. the
  /// former is used when the content type is `[u32]`, the latter is used when the heap is wrapped by
  /// the typed u32 heap ptr, which compares the u32 offset with the array length when the bound
  /// check is enabled.
  length_from_last_texel: bool,
}

impl TextureAsU32Heap {
  fn last_texel(&self) -> Node<u32> {
    let size = self.texture.texture_dimension_2d(None);
    let position = (size.x() - val(1), size.y() - val(1)).into();
    self.texture.load_texel(position, 0).x()
  }
}

impl AbstractShaderPtr for TextureAsU32Heap {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let (texel, component) = if self.u32_per_texel == 1 {
      (index, None)
    } else {
      let u32_per_texel = val(self.u32_per_texel);
      (index / u32_per_texel, Some(index % u32_per_texel))
    };
    let x = texel % self.width;
    let y = texel / self.width;
    Box::new(TextureAsU32HeapPosition {
      texture: self.texture,
      position: (x, y).into(),
      component,
    })
  }

  fn array_length(&self) -> Node<u32> {
    if self.length_from_last_texel {
      self.last_texel()
    } else {
      let height = self.texture.texture_dimension_2d(None).y();
      (self.width * height - val(1)) * val(self.u32_per_texel)
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
  /// None for the single component texel
  component: Option<Node<u32>>,
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
    let texel = self.texture.load_texel(self.position, 0);
    let value = match self.component {
      Some(component) => texel.index(component),
      None => texel.x(),
    };
    value.handle()
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
