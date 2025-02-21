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

// todo, improve clone performance, use Arc
#[derive(Clone)]
pub struct U32BufferLoadStoreSourceWithType {
  pub ptr: U32BufferLoadStoreSource,
  pub ty: ShaderValueSingleType,
  pub bind_index: u32,
  pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct UnsizedArrayRepr {
  item_ty: ShaderU32TypeReprSingle,
  binding_index: u32,
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
    self.ty_mapping.get(name).map(|v| v.u32_count).unwrap()
  }
  pub fn get_struct_sub_field_u32_offset_ty(
    &self,
    struct_name: &str,
    field_idx: usize,
  ) -> (u32, &ShaderU32TypeReprSingle) {
    self
      .ty_mapping
      .get(name)
      .map(|v| v.sub_field_u32_offsets[field_idx])
      .unwrap()
  }
}

impl AbstractShaderPtr for U32BufferLoadStoreSourceWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    let meta = self.meta.read();
    if let ShaderU32TypeReprMaybeArrayed::Single(single) = &self.ty {
      match single {
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
        ShaderU32TypeReprSingle::Struct { tid, .. } => {
          let (offset, ty) = meta.get_struct_sub_field_u32_offset_ty(*tid, field_index);
          Box::new(Self {
            ptr: self.ptr.advance(offset),
            ty: ShaderU32TypeReprMaybeArrayed::Single(*ty),
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
    if let ShaderU32TypeReprMaybeArrayed::UnsizedArray(ty) = &self.ty {
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
    if let ShaderU32TypeReprMaybeArrayed::UnsizedArray(UnsizedArrayRepr {
      binding_index,
      item_ty,
    }) = &self.ty
    {
      let sub_buffer_u32_length = self.ptr.array.index(*binding_index + 1).load();
      let width = item_ty.u32_count(&meta);
      sub_buffer_u32_length / val(width as u32)
    } else {
      unreachable!("not an runtime-size array type")
    }
  }

  fn load(&self) -> ShaderNodeRawHandle {
    let meta = self.meta.read();

    use ShaderU32TypeReprMaybeArrayed::*;
    match &self.ty {
      Single(ty) => load_impl(&self.ptr.array, self.ptr.offset, ty, &meta),
      FixedSizeArray(shader_u32_type_repr_single, len) => {
        let step_size: u32 = todo!();
        let mut offset = self.ptr.offset;
        let parameters = (0..*len)
          .map(|v| {
            offset += val(step_size);
            load_impl(&self.ptr.array, offset, shader_u32_type_repr_single, &meta)
          })
          .collect();
        ShaderNodeExpr::Compose {
          target: ShaderSizedValueType::FixedSizeArray(todo!(), *len),
          parameters,
        }
        .insert_api_raw()
      }
      _ => {
        unreachable!("can not load unsized value")
      }
    }
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    let meta = self.meta.read();

    use ShaderU32TypeReprMaybeArrayed::*;
    match &self.ty {
      Single(shader_u32_type_repr_single) => todo!(),
      FixedSizeArray(shader_u32_type_repr_single, _) => todo!(),
      UnsizedArray(_) | UnsizedStruct { .. } => {
        unreachable!("can not store unsized value")
      }
    }

    fn store_impl(
      src: ShaderNodeRawHandle,
      dst: &ShaderPtrOf<[u32]>,
      offset: Node<u32>,
      ty: &ShaderU32TypeReprSingle,
      meta: &ShaderU32StructMetaData,
    ) {

      //
    }

    todo!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    todo!() // consider us dedicate atomic u32 heap.

    // let meta = self.meta.read();
    // // todo, assert self array is atomic u32[]
    // if let ShaderU32TypeReprMaybeArrayed::Single(ShaderU32TypeReprSingle::Atomic(_)) = &self.ty {
    //   let atomic = self.ptr.array.index(self.ptr.offset);
    //   atomic.get_raw_ptr().get_self_atomic_ptr()
    // } else {
    //   unreachable!("not an atomic type")
    // }
  }
}

fn load_impl(
  src: &ShaderPtrOf<[u32]>,
  mut offset: Node<u32>,
  ty: &ShaderU32TypeReprSingle,
  meta: &ShaderU32StructMetaData,
) -> ShaderNodeRawHandle {
  match ty {
    ShaderU32TypeReprSingle::Primitive(p) => {
      let size = ShaderSizedValueType::Primitive(*p).u32_size_count();
      let mut parameters = Vec::new();
      for _ in 0..size {
        let u32_read = src.index(offset).load();
        offset += val(1);
        let handle = ShaderNodeExpr::Convert {
          source: u32_read.handle(),
          convert_to: p.channel_ty(),
          convert: None,
        }
        .insert_api_raw();
        parameters.push(handle);
      }

      if let Some((mat_row, row_ty)) = p.mat_row_info() {
        let mut parameter_row = Vec::with_capacity(mat_row);
        for sub_parameters in parameters.chunks_exact(mat_row) {
          let mut parameters = sub_parameters.to_vec();
          if !matches!(meta.layout, VirtualShaderTypeLayout::Packed)
            && matches!(p, PrimitiveShaderValueType::Mat3Float32)
          {
            parameters.pop();
          }
          parameter_row.push(
            ShaderNodeExpr::Compose {
              target: row_ty.clone(),
              parameters,
            }
            .insert_api_raw(),
          )
        }
        parameters = parameter_row;
      }

      if parameters.len() == 1 {
        parameters[0]
      } else {
        ShaderNodeExpr::Compose {
          target: ShaderSizedValueType::Primitive(*p),
          parameters,
        }
        .insert_api_raw()
      }
    }
    ShaderU32TypeReprSingle::Struct { tid, field_count } => {
      let (_, _, struct_ty) = &meta.struct_mapping[*tid];
      let base_offset = offset;
      let sub_field_nodes = (0..*field_count)
        .map(|i| {
          let (sub_offset, ty) = meta.get_struct_sub_field_u32_offset_ty(*tid, i);
          let field_start = base_offset + val(sub_offset);
          load_impl(src, field_start, ty, meta)
        })
        .collect::<Vec<_>>();
      ShaderNodeExpr::Compose {
        target: ShaderSizedValueType::Struct(struct_ty.clone()),
        parameters: sub_field_nodes.to_vec(),
      }
      .insert_api_raw()
    }
  }
}
