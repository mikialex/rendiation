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

pub struct U32BufferLoadStore<T> {
  pub accessor: U32BufferLoadStoreSource,
  pub ty: PhantomData<T>,
}

impl<T> ShaderAbstractLeftValue for U32BufferLoadStore<T>
where
  T: ShaderSizedValueNodeType,
{
  type RightValue = Node<T>;

  fn abstract_load(&self) -> Self::RightValue {
    Node::<T>::load_from_u32_buffer(&self.accessor.array, self.accessor.offset)
  }

  fn abstract_store(&self, payload: Node<T>) {
    payload.store_into_u32_buffer(&self.accessor.array, self.accessor.offset);
  }
}

#[derive(Clone)]
pub struct U32BufferLoadStoreSourceWithType {
  pub ptr: U32BufferLoadStoreSource,
  pub ty: usize,
  pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderU32TypeReprSingle {
  Atomic(ShaderAtomicValueType),
  Primitive(PrimitiveShaderValueType),
  Struct(usize),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderU32TypeReprMaybeArrayed {
  Single(ShaderU32TypeReprSingle),
  FixedSizeArray(ShaderU32TypeReprSingle, usize),
  UnsizedArray(UnsizedArrayRepr),
  UnsizedStruct {
    ty: usize,
    tail_unsized_array: UnsizedArrayRepr,
  },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct UnsizedArrayRepr {
  item_ty: ShaderU32TypeReprSingle,
  binding_index: u32,
}

pub struct ShaderU32StructMetaData {
  types: Vec<ShaderU32TypeReprMaybeArrayed>,
  size_and_offsets: Vec<u32>,
  sub_field_ty: Vec<usize>,
  struct_mapping: Vec<(usize, usize)>,
  ty_mapping: FastHashMap<ShaderSizedValueType, usize>,
  layout: VirtualShaderTypeLayout,
}

/// implementation note: in the future we may using `vec4<f32>` heap instead of u32 to enable
/// vectorized load to improve performance. to implement this, packed layout will not be supported
/// because it will require `vec4<f32>` sized alignment.
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
    // todo populate default primitive types
    let mut v = Self {
      types: Default::default(),
      size_and_offsets: Default::default(),
      sub_field_ty: Default::default(),
      struct_mapping: Default::default(),
      ty_mapping: Default::default(),
      layout,
    };
    // v.register_ty(ShaderSizedValueType::Atomic(ShaderAtomicValueType::U32));
    // v.register_ty(ShaderSizedValueType::Atomic(ShaderAtomicValueType::I32));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Bool,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Vec2Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Vec3Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Vec4Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Mat2Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Mat3Float32,
    // ));
    // v.register_ty(ShaderSizedValueType::Primitive(
    //   PrimitiveShaderValueType::Mat4Float32,
    // ));

    v
  }
}

impl ShaderU32StructMetaData {
  pub fn register_ty(&mut self, ty: MaybeUnsizedValueType) -> usize {
    // self.struct_mapping.en
    todo!()
  }
  pub fn get_struct_u32_size(&self, struct_idx: usize) -> u32 {
    let (start, _) = self.struct_mapping[struct_idx];
    self.size_and_offsets[start]
  }
  pub fn get_struct_sub_field_u32_offset_ty(
    &self,
    struct_idx: usize,
    field_idx: usize,
  ) -> (u32, usize) {
    let (start, sub_type_start) = self.struct_mapping[struct_idx];
    let offset = self.size_and_offsets[start + field_idx + 1];
    let sub_type = self.sub_field_ty[sub_type_start + field_idx];
    (offset, sub_type)
  }
}

impl AbstractShaderPtr for U32BufferLoadStoreSourceWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let meta = self.meta.read();
    let ty = &meta.types[self.ty];
    if let ShaderU32TypeReprMaybeArrayed::Single(single) = ty {
      match single {
        ShaderU32TypeReprSingle::Atomic(_) => unreachable!("atomic does not have fields"),
        ShaderU32TypeReprSingle::Primitive(primitive_shader_value_type) => {
          use PrimitiveShaderValueType::*;
          let offset = match primitive_shader_value_type {
            Bool | Int32 | Float32 => unreachable!("single primitive does not have fields"),
            Mat2Float32 => 2,
            Mat3Float32 => 3,
            Mat4Float32 => 4,
            _ => field_index as u32,
          };
          Box::new(Self {
            ptr: self.ptr.advance(offset),
            ty: todo!(),
            meta: self.meta.clone(),
          })
        }
        ShaderU32TypeReprSingle::Struct(idx) => {
          let (offset, ty) = meta.get_struct_sub_field_u32_offset_ty(*idx, field_index);
          Box::new(Self {
            ptr: self.ptr.advance(offset),
            ty,
            meta: self.meta.clone(),
          })
        }
      }
    } else {
      unreachable!("array type can not be static indexed")
    }
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let meta = self.meta.read();
    let ty = meta.types[self.ty];
    if let ShaderU32TypeReprMaybeArrayed::UnsizedArray(ty) = ty {
      // note, the array bound check will be done automatically at outside if enabled.
      let size = meta.get_struct_u32_size(todo!());
      Box::new(Self {
        ptr: self.ptr.advance(val(size) * index),
        ty: todo!(),
        meta: self.meta.clone(),
      })
    } else {
      unreachable!("not an runtime-size array type")
    }
  }

  fn array_length(&self) -> Node<u32> {
    let meta = self.meta.read();
    let ty = &meta.types[self.ty];
    if let ShaderU32TypeReprMaybeArrayed::UnsizedArray(UnsizedArrayRepr { binding_index, .. }) = ty
    {
      let sub_buffer_u32_length = self.ptr.array.index(*binding_index + 1).load();
      todo!() // div by ty u32 count
    } else {
      unreachable!("not an runtime-size array type")
    }
  }

  fn load(&self) -> ShaderNodeRawHandle {
    todo!()
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    todo!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    let meta = self.meta.read();
    let ty = &meta.types[self.ty];
    // todo, assert self array is atomic u32[]
    if let ShaderU32TypeReprMaybeArrayed::Single(ShaderU32TypeReprSingle::Atomic(_)) = ty {
      let atomic = self.ptr.array.index(self.ptr.offset);
      atomic.get_raw_ptr().get_self_atomic_ptr()
    } else {
      unreachable!("not an atomic type")
    }
  }
}
