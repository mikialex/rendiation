use crate::*;

#[derive(Clone)]
pub struct U32BufferLoadStoreSource {
  /// internal structure when used as the implementation of AbstractShaderPtr
  /// ```
  /// [
  ///   u32: how many unsized array does this combine buffer contains
  ///   *u32: these unsized array's array length
  ///   *u32: real data
  /// ]
  /// ```
  pub array: ShaderPtrOf<[u32]>,
  pub offset: Node<u32>,
}

impl U32BufferLoadStoreSource {
  pub fn advance(&self, u32_offset: impl Into<Node<u32>>) -> Self {
    Self {
      array: self.array.clone(),
      offset: self.offset + u32_offset.into(),
    }
  }
}

pub struct U32BufferLoadStorePacked<T> {
  pub accessor: U32BufferLoadStoreSource,
  pub ty: PhantomData<T>,
}

impl<T> ShaderAbstractLeftValue for U32BufferLoadStorePacked<T>
where
  T: ShaderSizedValueNodeType,
{
  type RightValue = Node<T>;

  fn abstract_load(&self) -> Self::RightValue {
    Node::<T>::load_from_u32_buffer(
      &self.accessor.array,
      self.accessor.offset,
      VirtualShaderTypeLayout::Packed,
    )
  }

  fn abstract_store(&self, payload: Node<T>) {
    payload.store_into_u32_buffer(
      &self.accessor.array,
      self.accessor.offset,
      VirtualShaderTypeLayout::Packed,
    );
  }
}

// todo, improve clone performance, use Arc
#[derive(Clone)]
pub struct U32BufferLoadStoreSourceWithType {
  pub ptr: U32BufferLoadStoreSource,
  pub ty: ShaderValueSingleType,
  pub bind_index: u32,
  pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
}

pub struct ShaderU32StructMetaData {
  ty_mapping: FastHashMap<String, StructPrecomputeOffsetMetaData>,
  layout: VirtualShaderTypeLayout,
}

struct StructPrecomputeOffsetMetaData {
  u32_count: u32,
  sub_field_u32_offsets: Vec<u32>,
}

/// implementation note: in the future we may using `vec4<f32>` heap instead of u32 to enable
/// vectorized load to improve performance. to implement this, packed layout will not be supported
/// because it will require `vec4<f32>` sized alignment.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum VirtualShaderTypeLayout {
  /// most memory efficient, use this if no host side interaction is required
  Packed,
  /// match the uniform layout for host data exchange
  Std140,
  /// match the storage layout for host data exchange
  Std430,
}

impl ShaderU32StructMetaData {
  pub fn new(layout: VirtualShaderTypeLayout) -> Self {
    Self {
      ty_mapping: Default::default(),
      layout,
    }
  }
}

impl ShaderU32StructMetaData {
  pub fn register_ty(&mut self, ty: &MaybeUnsizedValueType) {
    match ty {
      MaybeUnsizedValueType::Sized(shader_sized_value_type) => todo!(),
      MaybeUnsizedValueType::Unsized(shader_un_sized_value_type) => todo!(),
    }
  }
  pub fn get_struct_u32_size(&self, struct_name: &str) -> u32 {
    self
      .ty_mapping
      .get(struct_name)
      .map(|v| v.u32_count)
      .unwrap()
  }
  pub fn get_struct_sub_field_u32_offset(&self, struct_name: &str, field_idx: usize) -> u32 {
    self
      .ty_mapping
      .get(struct_name)
      .map(|v| v.sub_field_u32_offsets[field_idx])
      .unwrap()
  }
}

impl AbstractShaderPtr for U32BufferLoadStoreSourceWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let meta = self.meta.read();
    let err = "unsupported sub field access target";
    let ptr = match &self.ty {
      ShaderValueSingleType::Sized(ty) => match ty {
        ShaderSizedValueType::Primitive(ty) => {
          use PrimitiveShaderValueType::*;
          let offset = match ty {
            Bool | Int32 | Float32 => unreachable!("single primitive does not have fields"),
            Mat2Float32 => 2,
            Mat3Float32 => 3,
            Mat4Float32 => 4,
            _ => field_index as u32,
          };
          Self {
            ptr: self.ptr.advance(offset),
            ty: todo!(),
            meta: self.meta.clone(),
            bind_index: self.bind_index,
          }
        }
        ShaderSizedValueType::Struct(ty) => {
          let offset = meta.get_struct_sub_field_u32_offset(&ty.name, field_index);
          Self {
            ptr: self.ptr.advance(offset),
            ty: ShaderValueSingleType::Sized(ty.fields[field_index].ty.clone()),
            meta: self.meta.clone(),
            bind_index: self.bind_index,
          }
        }
        ShaderSizedValueType::FixedSizeArray(ty, _) => todo!(),
        _ => unreachable!("{err}"),
      },
      ShaderValueSingleType::Unsized(ty) => match ty {
        ShaderUnSizedValueType::UnsizedArray(ty) => unreachable!("{err}"),
        ShaderUnSizedValueType::UnsizedStruct(ty) => {
          //
          todo!()
        }
      },
      _ => unreachable!("{err}"),
    };
    Box::new(ptr)
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = &self.ty {
      // note, the array bound check will be done automatically at outside if enabled.
      let size: u32 = todo!();
      Box::new(Self {
        ptr: self.ptr.advance(val(size) * index),
        ty: ShaderValueSingleType::Sized(**ty),
        meta: self.meta.clone(),
        bind_index: self.bind_index,
      })
    } else {
      unreachable!("not an runtime-size array type")
    }
  }

  fn array_length(&self) -> Node<u32> {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = &self.ty {
      let sub_buffer_u32_length = self.ptr.array.index(self.bind_index + 1).load();
      let width = ty.u32_size_count(meta.layout);
      sub_buffer_u32_length / val(width as u32)
    } else {
      unreachable!("not an runtime-size array type")
    }
  }

  fn load(&self) -> ShaderNodeRawHandle {
    let meta = self.meta.read(); // todo layout
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      ty.load_from_u32_buffer(&self.ptr.array, self.ptr.offset, meta.layout)
    } else {
      unreachable!("can not load unsized ty")
    }
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    let meta = self.meta.read(); // todo layout
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      ty.store_into_u32_buffer(value, &self.ptr.array, self.ptr.offset, meta.layout)
    } else {
      unreachable!("can not store unsized ty")
    }
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    todo!() // consider us dedicate atomic u32 heap.

    // if let ShaderValueSingleType::Sized(ShaderSizedValueType::Atomic(_)) = &self.ty {
    //   // let atomic = self.ptr.array.index(self.ptr.offset);
    //   // atomic.get_self_atomic_ptr()
    // }else{
    //   unreachable!("not an atomic type")
    // }
  }
}
