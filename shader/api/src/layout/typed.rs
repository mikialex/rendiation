use crate::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StructLayoutTarget {
  Std140,
  Std430,
}

pub fn align_of_struct_sized_fields(
  fields: &[ShaderStructFieldMetaInfo],
  target: StructLayoutTarget,
) -> usize {
  let align = fields
    .iter()
    .map(|field| field.ty.align_of_self(target))
    .max()
    .unwrap_or(1);

  match target {
    StructLayoutTarget::Std140 => round_up(16, align),
    StructLayoutTarget::Std430 => align,
  }
}

pub fn size_of_struct_sized_fields(
  fields: &[ShaderStructFieldMetaInfo],
  target: StructLayoutTarget,
) -> usize {
  let mut offset = 0;
  for (index, field) in fields.iter().enumerate() {
    let size = field.ty.size_of_self(target);
    let alignment = if index + 1 == fields.len() {
      align_of_struct_sized_fields(fields, target)
    } else {
      fields[index + 1].ty.align_of_self(target)
    };
    offset += size;
    let pad_size = align_offset(offset, alignment);
    offset += pad_size;
  }
  let size = offset;

  // we always make sure the struct size is round up to struct align, this is different!
  match target {
    StructLayoutTarget::Std140 => round_up(16, size),
    StructLayoutTarget::Std430 => size,
  }
}

impl ShaderStructMetaInfo {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    align_of_struct_sized_fields(&self.fields, target)
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    size_of_struct_sized_fields(&self.fields, target)
  }
}

/// Round `n` up to the nearest alignment boundary.
pub fn round_up(k: usize, n: usize) -> usize {
  // equivalent to:
  // match n % k {
  //     0 => n,
  //     rem => n + (k - rem),
  // }
  let mask = k - 1;
  (n + mask) & !mask
}

impl ShaderSizedValueType {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      ShaderSizedValueType::Atomic(t) => t.align_of_self(),
      ShaderSizedValueType::Primitive(t) => t.align_of_self(),
      ShaderSizedValueType::Struct(t) => t.align_of_self(target),
      ShaderSizedValueType::FixedSizeArray(t, _) => {
        let align = t.align_of_self(target);
        match target {
          StructLayoutTarget::Std140 => round_up(16, align),
          StructLayoutTarget::Std430 => align,
        }
      }
    }
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      ShaderSizedValueType::Atomic(t) => t.size_of_self(),
      ShaderSizedValueType::Primitive(t) => t.size_of_self(),
      ShaderSizedValueType::Struct(t) => t.size_of_self(target),
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        size * round_up(self.align_of_self(target), ty.size_of_self(target))
      }
    }
  }
}

impl ShaderAtomicValueType {
  pub fn align_of_self(&self) -> usize {
    match self {
      ShaderAtomicValueType::I32 => 4,
      ShaderAtomicValueType::U32 => 4,
    }
  }

  pub fn size_of_self(&self) -> usize {
    match self {
      ShaderAtomicValueType::I32 => 4,
      ShaderAtomicValueType::U32 => 4,
    }
  }
}

impl PrimitiveShaderValueType {
  /// for type that not host-shareable (e.g. bool related), assume u32 equivalent is used.
  pub fn align_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 4,
      PrimitiveShaderValueType::Int32 => 4,
      PrimitiveShaderValueType::Uint32 => 4,
      PrimitiveShaderValueType::Float32 => 4,
      PrimitiveShaderValueType::Vec2Bool => 8,
      PrimitiveShaderValueType::Vec3Bool => 16,
      PrimitiveShaderValueType::Vec4Bool => 16,
      PrimitiveShaderValueType::Vec2Float32 => 8,
      PrimitiveShaderValueType::Vec3Float32 => 16,
      PrimitiveShaderValueType::Vec4Float32 => 16,
      PrimitiveShaderValueType::Mat2Float32 => 8,
      PrimitiveShaderValueType::Mat3Float32 => 16,
      PrimitiveShaderValueType::Mat4Float32 => 16,
      PrimitiveShaderValueType::Mat4x3Float32 => 16,
      PrimitiveShaderValueType::Vec2Uint32 => 8,
      PrimitiveShaderValueType::Vec3Uint32 => 16,
      PrimitiveShaderValueType::Vec4Uint32 => 16,
      PrimitiveShaderValueType::Vec2Int32 => 8,
      PrimitiveShaderValueType::Vec3Int32 => 16,
      PrimitiveShaderValueType::Vec4Int32 => 16,
    }
  }

  pub fn size_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 4,
      PrimitiveShaderValueType::Int32 => 4,
      PrimitiveShaderValueType::Uint32 => 4,
      PrimitiveShaderValueType::Float32 => 4,
      PrimitiveShaderValueType::Vec2Bool => 8,
      PrimitiveShaderValueType::Vec3Bool => 12,
      PrimitiveShaderValueType::Vec4Bool => 16,
      PrimitiveShaderValueType::Vec2Float32 => 8,
      PrimitiveShaderValueType::Vec3Float32 => 12,
      PrimitiveShaderValueType::Vec4Float32 => 16,
      PrimitiveShaderValueType::Vec2Uint32 => 8,
      PrimitiveShaderValueType::Vec3Uint32 => 12,
      PrimitiveShaderValueType::Vec4Uint32 => 16,
      PrimitiveShaderValueType::Vec2Int32 => 8,
      PrimitiveShaderValueType::Vec3Int32 => 12,
      PrimitiveShaderValueType::Vec4Int32 => 16,
      PrimitiveShaderValueType::Mat2Float32 => 16,
      PrimitiveShaderValueType::Mat3Float32 => 48,
      PrimitiveShaderValueType::Mat4Float32 => 64,
      PrimitiveShaderValueType::Mat4x3Float32 => 48,
    }
  }
}
