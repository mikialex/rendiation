use crate::*;

pub trait RawBufferSerialization {
  fn u32_size_count() -> u32;
  fn load_from_u32_buffer(target: StorageNode<[u32]>, offset: Node<u32>) -> Self;
  fn store_into_u32_buffer(self, target: StorageNode<[u32]>, offset: Node<u32>);
}

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

  pub fn mat_row(&self) -> Option<usize> {
    match self {
      PrimitiveShaderValueType::Mat2Float32 => Some(2),
      PrimitiveShaderValueType::Mat3Float32 => Some(3),
      PrimitiveShaderValueType::Mat4Float32 => Some(4),
      _ => None,
    }
  }
}

impl ShaderSizedValueType {
  pub fn u32_size_count(&self) -> u32 {
    match self {
      ShaderSizedValueType::Atomic(_) => 1,
      ShaderSizedValueType::Primitive(p) => p.size_of_self() as u32 / 4,
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

  pub fn load_from_u32_buffer(&self, target: StorageNode<[u32]>, offset: Node<u32>) -> NodeUntyped {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to load from buffer"),
      ShaderSizedValueType::Primitive(p) => {
        let size = self.u32_size_count();
        let mut parameters = Vec::new();
        for _ in 0..size {
          let u32_read = target.index(offset).load();
          let converted = ShaderNodeExpr::Convert {
            source: u32_read.handle(),
            convert_to: p.channel_ty(),
            convert: None,
          }
          .insert_api::<AnyType>();
          parameters.push(converted.handle());
        }

        if let Some(mat_row) = p.mat_row() {
          let mut parameter_row = Vec::with_capacity(mat_row);
          for sub_parameters in parameters.chunks_exact(mat_row) {
            parameter_row.push(
              ShaderNodeExpr::Compose {
                target: self.clone(),
                parameters: sub_parameters.to_vec(),
              }
              .insert_api::<AnyType>()
              .handle(),
            )
          }
          parameters = parameter_row;
        }

        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api()
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
    source: NodeUntyped,
    target: StorageNode<[u32]>,
    mut offset: Node<u32>,
  ) {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to store into buffer"),
      ShaderSizedValueType::Primitive(p) => {
        fn index_and_write(
          target: StorageNode<[u32]>,
          offset: Node<u32>,
          source: NodeUntyped,
          idx: u32,
        ) {
          let channel =
            unsafe { index_access_field::<AnyType>(source.handle(), idx as usize).handle() };
          let converted = ShaderNodeExpr::Convert {
            source: channel,
            convert_to: ValueKind::Uint,
            convert: None,
          }
          .insert_api();
          target.index(offset).store(converted);
        }

        if let Some(mat_row) = p.mat_row() {
          for i in 0..mat_row {
            let row = unsafe { index_access_field::<AnyType>(source.handle(), i) };
            for j in 0..mat_row {
              index_and_write(target, offset, row, j as u32);
            }
          }
        } else {
          for i in 0..self.u32_size_count() {
            index_and_write(target, offset, source, i);
            offset += val(1);
          }
        }
      }
      ShaderSizedValueType::Struct(f) => {
        for (i, field) in f.fields.iter().enumerate() {
          field.ty.store_into_u32_buffer(
            unsafe { index_access_field(source.handle(), i) },
            target,
            offset,
          );
          offset += val(field.ty.u32_size_count());
        }
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let stride = val(ty.u32_size_count());
        for i in 0..*size {
          ty.store_into_u32_buffer(
            unsafe { index_access_field(source.handle(), i) },
            target,
            offset,
          );
          offset += stride;
        }
      }
    }
  }
}

impl<T: ShaderSizedValueNodeType> RawBufferSerialization for Node<T> {
  fn u32_size_count() -> u32 {
    T::sized_ty().u32_size_count()
  }

  fn load_from_u32_buffer(target: StorageNode<[u32]>, offset: Node<u32>) -> Self {
    unsafe {
      T::sized_ty()
        .load_from_u32_buffer(target, offset)
        .cast_type()
    }
  }

  fn store_into_u32_buffer(self, target: StorageNode<[u32]>, offset: Node<u32>) {
    T::sized_ty().store_into_u32_buffer(self.cast_untyped_node(), target, offset)
  }
}
