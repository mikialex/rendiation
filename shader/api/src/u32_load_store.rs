use crate::*;

#[derive(Clone)]
pub struct U32BufferLoadStoreSource {
  /// internal structure when used as the implementation of AbstractShaderPtr
  /// ```txt
  /// [
  ///   u32: how many subdata does this combine buffer contains
  ///   *u32: these subdata start offset in u32
  ///   *u32: these subdata length in u32
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
      &self.accessor.array.clone().into_readonly_view(),
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
/// implementation note: in the future we may use `vec4<f32>` heap instead of u32 to enable
/// vectorized load to improve performance. to implement this, packed layout will not be supported
/// because it will require `vec4<f32>` sized alignment.
#[derive(Clone)]
pub struct U32HeapPtrWithType {
  pub ptr: U32HeapPtr,
  pub ty: ShaderValueSingleType,
  pub array_length: Option<Node<u32>>, // only Some when it is a runtime-size array
  pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
}

pub struct ShaderU32StructMetaData {
  ty_mapping: FastHashMap<String, StructPrecomputeOffsetMetaData>,
  pub layout: StructLayoutTarget,
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
        ShaderUnSizedValueType::UnsizedStruct(ty) => self.register_unsized_struct(ty),
      },
    }
  }

  /// The runtime sized array is treated as the last field, so its offset can be accessed by the
  /// field index that equals to the sized field count.
  fn register_unsized_struct(&mut self, ty: &ShaderUnSizedStructMetaInfo) {
    let array_ty = &ty.last_dynamic_array_field.1;
    ty.sized_fields.iter().for_each(|f| {
      self.register_sized(&f.ty);
    });
    self.register_sized(array_ty);

    let layout = self.layout;
    self.ty_mapping.entry_ref(&ty.name).or_insert_with(|| {
      let mut sub_field_u32_offsets: Vec<_> =
        struct_fields_layout_by_rule(&ty.sized_fields, layout)
          .offsets
          .iter()
          .map(|byte_offset| {
            assert!(byte_offset.is_multiple_of(4));
            *byte_offset as u32 / 4
          })
          .collect();
      let (array_offset, _) = ty.runtime_array_layout(layout);
      assert!(array_offset.is_multiple_of(4));
      sub_field_u32_offsets.push(array_offset as u32 / 4);
      StructPrecomputeOffsetMetaData {
        // only the sized part is counted
        u32_count: array_offset as u32 / 4,
        sub_field_u32_offsets,
      }
    });
  }
  fn register_sized(&mut self, ty: &ShaderSizedValueType) {
    match ty {
      ShaderSizedValueType::Struct(ty) => self.register_struct(ty),
      ShaderSizedValueType::FixedSizeArray(ty, _) => self.register_sized(ty),
      _ => {}
    }
  }

  /// For std430 layout, the struct host layout is used if exists, so the host data is
  /// able to be shared with the shader directly.
  fn register_struct(&mut self, ty: &ShaderStructMetaInfo) {
    ty.fields.iter().for_each(|f| {
      self.register_sized(&f.ty);
    });

    let layout = self.layout;
    self.ty_mapping.entry_ref(&ty.name).or_insert_with(|| {
      let fields_layout = ty.fields_layout(layout);
      let sub_field_u32_offsets = fields_layout
        .offsets
        .iter()
        .map(|byte_offset| {
          assert!(byte_offset.is_multiple_of(4));
          *byte_offset as u32 / 4
        })
        .collect();
      assert!(fields_layout.size.is_multiple_of(4));
      StructPrecomputeOffsetMetaData {
        u32_count: fields_layout.size as u32 / 4,
        sub_field_u32_offsets,
      }
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
          let (offset, fty) = match ty {
            PrimitiveShaderValueType::Scalar(_) => {
              unreachable!("single primitive does not have fields")
            }
            PrimitiveShaderValueType::Vector { scalar, .. } => (
              field_index as u32,
              PrimitiveShaderValueType::Scalar(*scalar),
            ),
            PrimitiveShaderValueType::Matrix { rows, scalar, .. } => {
              let stride = matrix_column_stride(*rows, meta.layout) as u32 / 4;
              (
                stride * field_index as u32,
                PrimitiveShaderValueType::vector(*rows, *scalar),
              )
            }
          };
          Self {
            ptr: self.ptr.advance(offset),
            ty: ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(fty)),
            meta: self.meta.clone(),
            array_length: None,
          }
        }
        ShaderSizedValueType::Struct(ty) => {
          let offset = meta.get_struct_sub_field_u32_offset(&ty.name, field_index);
          Self {
            ptr: self.ptr.advance(offset),
            ty: ShaderValueSingleType::Sized(ty.fields[field_index].ty.clone()),
            meta: self.meta.clone(),
            array_length: None,
          }
        }
        ShaderSizedValueType::FixedSizeArray(ty, _) => {
          let stride = array_stride_of_element(ty, meta.layout) as u32 / 4;
          Self {
            ptr: self.ptr.advance(val(stride) * val(field_index as u32)),
            ty: ShaderValueSingleType::Sized((**ty).clone()),
            meta: self.meta.clone(),
            array_length: None,
          }
        }
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
            array_length: None,
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
      let stride = array_stride_of_element(ty, meta.layout) as u32 / 4;
      Box::new(Self {
        ptr: self.ptr.advance(val(stride) * index),
        ty: ShaderValueSingleType::Sized((**ty).clone()),
        meta: self.meta.clone(),
        array_length: None,
      })
    } else {
      // todo, we should support fixed size array, as the rrf shader api allows user do field_array_index on fixed size array
      // !but not on (mat vec struct). the difference between the field_array_index and field_index is the dynamistic of index.
      // so the function name should be fixed as well.
      unreachable!("not an runtime-size array type")
    }
  }

  fn array_length(&self) -> Node<u32> {
    self
      .array_length
      .expect("not an runtime-size array type or unsupported unsized struct")
  }

  fn load(&self) -> ShaderNodeRawHandle {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      let array = self.ptr.array.downcast_as_common_u32_buffer();
      ty.load_from_u32_buffer(
        &array.clone().into_readonly_view(),
        self.ptr.offset,
        meta.layout,
      )
    } else {
      unreachable!("can not load unsized ty")
    }
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    let meta = self.meta.read();
    if let ShaderValueSingleType::Sized(ty) = &self.ty {
      let array = self.ptr.array.downcast_as_common_u32_buffer();
      ty.store_into_u32_buffer(value, array, self.ptr.offset, meta.layout)
    } else {
      unreachable!("can not store unsized ty")
    }
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    if let ShaderValueSingleType::Sized(ShaderSizedValueType::Atomic(_)) = &self.ty {
      self.ptr.array.get_single_atomic_at(self.ptr.offset)
    } else {
      unreachable!("self is not an atomic type")
    }
  }
  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!("can not get raw ptr of u32 heap ptr")
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
}

impl U32HeapHeapSource {
  pub fn downcast_as_common_u32_buffer(&self) -> &ShaderPtrOf<[u32]> {
    match &self {
      U32HeapHeapSource::Common(ptr) => ptr,
      _ => unreachable!("failed to downcast as common u32 buffer"),
    }
  }
  /// note, when using atomic i32, the value will be bitcast to u32
  pub fn bitcast_read_u32_at(&self, index: impl Into<Node<u32>>) -> Node<u32> {
    let index = index.into();
    match &self {
      U32HeapHeapSource::Common(ptr) => ptr.index(index).load(),
      U32HeapHeapSource::AtomicU32(ptr) => ptr.index(index).atomic_load(),
      U32HeapHeapSource::AtomicI32(ptr) => ptr.index(index).atomic_load().bitcast(),
    }
  }
  pub fn get_single_atomic_at(&self, index: impl Into<Node<u32>>) -> ShaderNodeRawHandle {
    let index = index.into();
    match &self {
      U32HeapHeapSource::Common(_) => unreachable!("heap is not atomic buffer"),
      U32HeapHeapSource::AtomicU32(ptr) => ptr.index(index).get_raw_ptr().get_self_atomic_ptr(),
      U32HeapHeapSource::AtomicI32(ptr) => ptr.index(index).get_raw_ptr().get_self_atomic_ptr(),
    }
  }
}

#[test]
fn unsized_struct_runtime_array_offset() {
  let ty: &'static _ = Box::leak(Box::new(ShaderUnSizedStructMetaInfo {
    name: "UnsizedStructForTest".into(),
    sized_fields: vec![ShaderStructFieldMetaInfo {
      name: "count".into(),
      ty: u32::sized_ty(),
      ty_deco: None,
    }],
    last_dynamic_array_field: ("data".into(), Box::new(Vec4::<f32>::sized_ty())),
  }));
  let ty = MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedStruct(ty));

  let expect = [
    (StructLayoutTarget::Std430, 4),
    (StructLayoutTarget::Packed, 1),
  ];
  for (layout, array_u32_offset) in expect {
    let mut meta = ShaderU32StructMetaData::new(layout);
    meta.register_ty(&ty);
    assert_eq!(
      meta.get_struct_sub_field_u32_offset("UnsizedStructForTest", 0),
      0
    );
    assert_eq!(
      meta.get_struct_sub_field_u32_offset("UnsizedStructForTest", 1),
      array_u32_offset
    );
  }
}
