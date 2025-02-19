use crate::*;

impl PrimitiveShaderValueType {
  pub fn channel_ty(&self) -> ValueKind {
    match self {
      PrimitiveShaderValueType::Bool => ValueKind::Bool,
      PrimitiveShaderValueType::Int32 => ValueKind::Int,
      PrimitiveShaderValueType::Uint32 => ValueKind::Uint,
      PrimitiveShaderValueType::Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Vec2Bool => ValueKind::Bool,
      PrimitiveShaderValueType::Vec3Bool => ValueKind::Bool,
      PrimitiveShaderValueType::Vec4Bool => ValueKind::Bool,
      PrimitiveShaderValueType::Vec2Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Vec3Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Vec4Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Vec2Uint32 => ValueKind::Uint,
      PrimitiveShaderValueType::Vec3Uint32 => ValueKind::Uint,
      PrimitiveShaderValueType::Vec4Uint32 => ValueKind::Uint,
      PrimitiveShaderValueType::Vec2Int32 => ValueKind::Int,
      PrimitiveShaderValueType::Vec3Int32 => ValueKind::Int,
      PrimitiveShaderValueType::Vec4Int32 => ValueKind::Int,
      PrimitiveShaderValueType::Mat2Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Mat3Float32 => ValueKind::Float,
      PrimitiveShaderValueType::Mat4Float32 => ValueKind::Float,
    }
  }

  pub fn u32_count_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 1,
      PrimitiveShaderValueType::Int32 => 1,
      PrimitiveShaderValueType::Uint32 => 1,
      PrimitiveShaderValueType::Float32 => 1,
      PrimitiveShaderValueType::Vec2Bool => 2,
      PrimitiveShaderValueType::Vec3Bool => 3,
      PrimitiveShaderValueType::Vec4Bool => 4,
      PrimitiveShaderValueType::Vec2Float32 => 2,
      PrimitiveShaderValueType::Vec3Float32 => 3,
      PrimitiveShaderValueType::Vec4Float32 => 4,
      PrimitiveShaderValueType::Vec2Uint32 => 2,
      PrimitiveShaderValueType::Vec3Uint32 => 3,
      PrimitiveShaderValueType::Vec4Uint32 => 4,
      PrimitiveShaderValueType::Vec2Int32 => 2,
      PrimitiveShaderValueType::Vec3Int32 => 3,
      PrimitiveShaderValueType::Vec4Int32 => 4,
      PrimitiveShaderValueType::Mat2Float32 => 4,
      PrimitiveShaderValueType::Mat3Float32 => 9,
      PrimitiveShaderValueType::Mat4Float32 => 16,
    }
  }

  pub fn is_single_primitive(&self) -> bool {
    self.u32_count_of_self() == 1
  }

  pub fn mat_row_info(&self) -> Option<(usize, ShaderSizedValueType)> {
    match self {
      PrimitiveShaderValueType::Mat2Float32 => (
        2,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec2Float32),
      ),
      PrimitiveShaderValueType::Mat3Float32 => (
        3,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec3Float32),
      ),
      PrimitiveShaderValueType::Mat4Float32 => (
        4,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec4Float32),
      ),
      _ => return None,
    }
    .into()
  }
}

impl ShaderSizedValueType {
  pub fn u32_size_count(&self) -> u32 {
    match self {
      ShaderSizedValueType::Atomic(_) => 1,
      ShaderSizedValueType::Primitive(p) => p.u32_count_of_self() as u32,
      ShaderSizedValueType::Struct(s) => {
        let mut size = 0;
        for field in &s.fields {
          size += field.ty.u32_size_count();
        }
        size
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => ty.u32_size_count() * *size as u32,
    }
  }

  pub fn load_from_u32_buffer(
    &self,
    target: &ShaderAccessorOf<[u32]>,
    mut offset: Node<u32>,
  ) -> NodeUntyped {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to load from buffer"),
      ShaderSizedValueType::Primitive(p) => {
        let size = ShaderSizedValueType::Primitive(*p).u32_size_count();
        let mut parameters = Vec::new();
        for _ in 0..size {
          let u32_read = target.index(offset).load();
          offset += val(1);
          let converted = ShaderNodeExpr::Convert {
            source: u32_read.handle(),
            convert_to: p.channel_ty(),
            convert: None,
          }
          .insert_api::<AnyType>();
          parameters.push(converted.handle());
        }

        if let Some((mat_row, row_ty)) = p.mat_row_info() {
          let mut parameter_row = Vec::with_capacity(mat_row);
          for sub_parameters in parameters.chunks_exact(mat_row) {
            parameter_row.push(
              ShaderNodeExpr::Compose {
                target: row_ty.clone(),
                parameters: sub_parameters.to_vec(),
              }
              .insert_api::<AnyType>()
              .handle(),
            )
          }
          parameters = parameter_row;
        }

        if parameters.len() == 1 {
          parameters[0].into_node_untyped()
        } else {
          ShaderNodeExpr::Compose {
            target: ShaderSizedValueType::Primitive(*p),
            parameters,
          }
          .insert_api()
        }
      }
      ShaderSizedValueType::Struct(f) => {
        let mut offset = offset;
        let mut parameters = Vec::new();
        for field in &f.fields {
          parameters.push(field.ty.load_from_u32_buffer(target, offset).handle());
          offset += val(field.ty.u32_size_count());
        }

        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api()
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let mut offset = offset;
        let stride = val(ty.u32_size_count());
        let mut parameters = Vec::new();
        for _ in 0..*size {
          parameters.push(ty.load_from_u32_buffer(target, offset).handle());
          offset += stride;
        }
        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api()
      }
    }
  }

  pub fn store_into_u32_buffer(
    &self,
    source: ShaderNodeRawHandle,
    target: &ShaderAccessorOf<[u32]>,
    mut offset: Node<u32>,
  ) {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to store into buffer"),
      ShaderSizedValueType::Primitive(p) => {
        fn index_and_write(
          target: &ShaderAccessorOf<[u32]>,
          offset: Node<u32>,
          source: ShaderNodeRawHandle,
          idx: Option<u32>,
        ) {
          let channel = if let Some(idx) = idx {
            unsafe { index_access_field(source, idx as usize) }
          } else {
            source
          };

          let converted = ShaderNodeExpr::Convert {
            source: channel,
            convert_to: ValueKind::Uint,
            convert: None,
          }
          .insert_api();
          target.index(offset).store(converted);
        }

        for i in 0..ShaderSizedValueType::Primitive(*p).u32_size_count() {
          let single = p.is_single_primitive();
          index_and_write(target, offset, source, (!single).then_some(i));
          offset += val(1);
        }
      }
      ShaderSizedValueType::Struct(f) => {
        for (i, field) in f.fields.iter().enumerate() {
          field
            .ty
            .store_into_u32_buffer(unsafe { index_access_field(source, i) }, target, offset);
          offset += val(field.ty.u32_size_count());
        }
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let stride = val(ty.u32_size_count());
        for i in 0..*size {
          ty.store_into_u32_buffer(unsafe { index_access_field(source, i) }, target, offset);
          offset += stride;
        }
      }
    }
  }
}

pub trait RawBufferSerializationExt {
  fn u32_size_count() -> u32;
  fn load_from_u32_buffer(target: &ShaderAccessorOf<[u32]>, offset: Node<u32>) -> Self;
  fn store_into_u32_buffer(self, target: &ShaderAccessorOf<[u32]>, offset: Node<u32>);
}

impl<T: ShaderSizedValueNodeType> RawBufferSerializationExt for Node<T> {
  fn u32_size_count() -> u32 {
    T::sized_ty().u32_size_count()
  }

  fn load_from_u32_buffer(target: &ShaderAccessorOf<[u32]>, offset: Node<u32>) -> Self {
    unsafe {
      T::sized_ty()
        .load_from_u32_buffer(target, offset)
        .cast_type()
    }
  }

  fn store_into_u32_buffer(self, target: &ShaderAccessorOf<[u32]>, offset: Node<u32>) {
    T::sized_ty().store_into_u32_buffer(self.handle(), target, offset)
  }
}
