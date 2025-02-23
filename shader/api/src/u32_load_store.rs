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
      StructLayoutTarget::Packed,
    )
  }

  fn abstract_store(&self, payload: Node<T>) {
    payload.store_into_u32_buffer(
      &self.accessor.array,
      self.accessor.offset,
      StructLayoutTarget::Packed,
    );
  }
}

// todo, improve clone performance, use Arc
//
/// implementation note: in the future we may using `vec4<f32>` heap instead of u32 to enable
/// vectorized load to improve performance. to implement this, packed layout will not be supported
/// because it will require `vec4<f32>` sized alignment.
#[derive(Clone)]
pub struct U32HeapPtrWithType {
  pub ptr: U32HeapPtr,
  pub ty: ShaderValueSingleType,
  pub bind_index: u32,
  pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
}

pub struct ShaderU32StructMetaData {
  ty_mapping: FastHashMap<String, StructPrecomputeOffsetMetaData>,
  layout: StructLayoutTarget,
}

struct StructPrecomputeOffsetMetaData {
  u32_count: u32,
  sub_field_u32_offsets: Vec<u32>,
}

impl ShaderU32StructMetaData {
  pub fn new(layout: StructLayoutTarget) -> Self {
    Self {
      ty_mapping: Default::default(),
      layout,
    }
  }
}

impl ShaderU32StructMetaData {
  pub fn register_ty(&mut self, ty: &MaybeUnsizedValueType) {
    match ty {
      MaybeUnsizedValueType::Sized(ty) => self.register_sized(ty),
      MaybeUnsizedValueType::Unsized(ty) => match ty {
        ShaderUnSizedValueType::UnsizedArray(ty) => self.register_sized(ty),
        ShaderUnSizedValueType::UnsizedStruct(ty) => {
          self.register_struct(&ty.name, &ty.sized_fields);
        }
      },
    }
  }
  fn register_sized(&mut self, ty: &ShaderSizedValueType) {
    match ty {
      ShaderSizedValueType::Struct(ty) => self.register_struct(&ty.name, &ty.fields),
      ShaderSizedValueType::FixedSizeArray(ty, _) => self.register_sized(ty),
      _ => {}
    }
  }

  fn register_struct(&mut self, struct_name: &str, fields: &[ShaderStructFieldMetaInfo]) {
    fields.iter().for_each(|f| {
      self.register_sized(&f.ty);
    });

    self
      .ty_mapping
      .raw_entry_mut()
      .from_key(struct_name)
      .or_insert_with(|| {
        let mut sub_field_u32_offsets = Vec::with_capacity(fields.len());
        let tail = iter_field_start_offset_in_bytes(fields, self.layout, &mut |byte_offset, _| {
          sub_field_u32_offsets.push(byte_offset as u32 / 4);
        });
        let struct_size = size_of_struct_sized_fields(fields, self.layout);
        assert!(tail.is_none());
        (
          struct_name.to_string(),
          StructPrecomputeOffsetMetaData {
            u32_count: struct_size as u32 / 4,
            sub_field_u32_offsets,
          },
        )
      });
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

impl AbstractShaderPtr for U32HeapPtrWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let meta = self.meta.read();
    let err = "unsupported sub field access target";
    let ptr = match &self.ty {
      ShaderValueSingleType::Sized(ty) => match ty {
        ShaderSizedValueType::Primitive(ty) => {
          use PrimitiveShaderValueType::*;
          let (offset, fty) = match ty {
            Bool | Int32 | Float32 => unreachable!("single primitive does not have fields"),
            Mat2Float32 => (2, Vec2Float32),
            Mat3Float32 => (
              if matches!(meta.layout, StructLayoutTarget::Packed) {
                3
              } else {
                4
              },
              Vec3Float32,
            ),
            Mat4Float32 => (4, Vec4Float32),
            _ => (
              field_index as u32,
              match ty {
                Vec2Bool | Vec3Bool | Vec4Bool => Bool,
                Vec2Float32 | Vec3Float32 | Vec4Float32 => Float32,
                Vec2Int32 | Vec3Int32 | Vec4Int32 => Int32,
                Vec2Uint32 | Vec3Uint32 | Vec4Uint32 => Uint32,
                _ => unreachable!(),
              },
            ),
          };
          Self {
            ptr: self.ptr.advance(offset),
            ty: ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(fty)),
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
        ShaderSizedValueType::FixedSizeArray(_, _) => todo!(),
        _ => unreachable!("{err}"),
      },
      ShaderValueSingleType::Unsized(ty) => match ty {
        ShaderUnSizedValueType::UnsizedArray(_) => unreachable!("{err}"),
        ShaderUnSizedValueType::UnsizedStruct(ty) => {
          let offset = meta.get_struct_sub_field_u32_offset(&ty.name, field_index);
          let ty = if field_index == ty.sized_fields.len() {
            ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedArray(
              ty.last_dynamic_array_field.1.clone(),
            ))
          } else {
            ShaderValueSingleType::Sized(ty.sized_fields[field_index].ty.clone())
          };
          Self {
            ptr: self.ptr.advance(offset),
            ty,
            meta: self.meta.clone(),
            bind_index: self.bind_index,
          }
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
      let size = ty.size_of_self(meta.layout) as u32 / 4;
      Box::new(Self {
        ptr: self.ptr.advance(val(size) * index),
        ty: ShaderValueSingleType::Sized((**ty).clone()),
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
      // we assume the host side will always write length in u32, so we get it from i32 by bitcast if needed
      let sub_buffer_u32_length = self.ptr.bitcast_read_u32_at(self.bind_index + 1);
      let width = ty.u32_size_count(meta.layout);
      sub_buffer_u32_length / val(width)
    } else {
      unreachable!("not an runtime-size array type or unsupported unsized struct")
    }
  }

  fn load(&self) -> ShaderNodeRawHandle {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      let array = self.ptr.downcast_as_common_u32_buffer();
      ty.load_from_u32_buffer(array, self.ptr.offset, meta.layout)
    } else {
      unreachable!("can not load unsized ty")
    }
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      let array = self.ptr.downcast_as_common_u32_buffer();
      ty.store_into_u32_buffer(value, array, self.ptr.offset, meta.layout)
    } else {
      unreachable!("can not store unsized ty")
    }
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    if let ShaderValueSingleType::Sized(ShaderSizedValueType::Atomic(_)) = &self.ty {
      self.ptr.get_single_atomic_at(self.ptr.offset)
    } else {
      unreachable!("self is not an atomic type")
    }
  }
}

#[derive(Clone)]
pub struct U32HeapPtr {
  pub array: U32HeapHeapSource,
  pub offset: Node<u32>,
}

#[derive(Clone)]
pub enum U32HeapHeapSource {
  Common(ShaderPtrOf<[u32]>),
  AtomicU32(ShaderPtrOf<[DeviceAtomic<u32>]>),
  AtomicI32(ShaderPtrOf<[DeviceAtomic<i32>]>),
}

impl U32HeapPtr {
  pub fn advance(&self, u32_offset: impl Into<Node<u32>>) -> Self {
    Self {
      array: self.array.clone(),
      offset: self.offset + u32_offset.into(),
    }
  }
  pub fn downcast_as_common_u32_buffer(&self) -> &ShaderPtrOf<[u32]> {
    match &self.array {
      U32HeapHeapSource::Common(ptr) => ptr,
      _ => unreachable!("failed to downcast as common u32 buffer"),
    }
  }
  /// note, when using atomic i32, the value will be bitcast to u32
  pub fn bitcast_read_u32_at(&self, index: impl Into<Node<u32>>) -> Node<u32> {
    let index = index.into();
    match &self.array {
      U32HeapHeapSource::Common(ptr) => ptr.index(index).load(),
      U32HeapHeapSource::AtomicU32(ptr) => ptr.index(index).atomic_load(),
      U32HeapHeapSource::AtomicI32(ptr) => ptr.index(index).atomic_load().bitcast(),
    }
  }
  pub fn get_single_atomic_at(&self, index: impl Into<Node<u32>>) -> ShaderNodeRawHandle {
    let index = index.into();
    match &self.array {
      U32HeapHeapSource::Common(_) => unreachable!("heap is not atomic buffer"),
      U32HeapHeapSource::AtomicU32(ptr) => ptr.index(index).get_raw_ptr().get_self_atomic_ptr(),
      U32HeapHeapSource::AtomicI32(ptr) => ptr.index(index).get_raw_ptr().get_self_atomic_ptr(),
    }
  }
}
