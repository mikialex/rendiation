use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum StructLayoutTarget {
  Std140,
  Std430,
}

impl ShaderStructMetaInfoOwned {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    let align = self
      .fields
      .iter()
      .map(|field| field.ty.align_of_self(target))
      .max()
      .unwrap_or(1);

    match target {
      StructLayoutTarget::Std140 => round_up(16, align),
      StructLayoutTarget::Std430 => align,
    }
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    let mut last_offset_and_size_of_member = None::<(usize, usize)>;

    for field in &self.fields {
      if let Some((last_offset, last_size)) = last_offset_and_size_of_member {
        last_offset_and_size_of_member = (
          round_up(field.ty.align_of_self(target), last_offset + last_size),
          field.ty.size_of_self(target),
        )
          .into();
      } else {
        last_offset_and_size_of_member = (0, field.ty.size_of_self(target)).into();
      }
    }

    round_up(
      last_offset_and_size_of_member.unwrap().0,
      self.align_of_self(target),
    )
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

impl ShaderStructMemberValueType {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      ShaderStructMemberValueType::Primitive(t) => t.align_of_self(),
      ShaderStructMemberValueType::Struct(t) => (*t).to_owned().align_of_self(target),
      ShaderStructMemberValueType::FixedSizeArray((t, _)) => {
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
      ShaderStructMemberValueType::Primitive(t) => t.size_of_self(),
      ShaderStructMemberValueType::Struct(t) => {
        let size = (*t).to_owned().size_of_self(target);
        // If a structure member itself has a structure type S, then the number of bytes between
        // the start of that member and the start of any following member must be at least roundUp(16, SizeOf(S)).
        match target {
          StructLayoutTarget::Std140 => round_up(16, size),
          StructLayoutTarget::Std430 => size,
        }
      }
      ShaderStructMemberValueType::FixedSizeArray((ty, size)) => {
        size * round_up(self.align_of_self(target), ty.size_of_self(target))
      }
    }
  }
}

impl PrimitiveShaderValueType {
  pub fn align_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 4,
      PrimitiveShaderValueType::Int32 => 4,
      PrimitiveShaderValueType::Uint32 => 4,
      PrimitiveShaderValueType::Float32 => 4,
      PrimitiveShaderValueType::Vec2Float32 => 8,
      PrimitiveShaderValueType::Vec3Float32 => 16,
      PrimitiveShaderValueType::Vec4Float32 => 16,
      PrimitiveShaderValueType::Mat2Float32 => 8,
      PrimitiveShaderValueType::Mat3Float32 => 16,
      PrimitiveShaderValueType::Mat4Float32 => 16,
      PrimitiveShaderValueType::Vec2Uint32 => 8,
      PrimitiveShaderValueType::Vec3Uint32 => 16,
      PrimitiveShaderValueType::Vec4Uint32 => 16,
    }
  }

  pub fn size_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 4,
      PrimitiveShaderValueType::Int32 => 4,
      PrimitiveShaderValueType::Uint32 => 4,
      PrimitiveShaderValueType::Float32 => 4,
      PrimitiveShaderValueType::Vec2Float32 => 8,
      PrimitiveShaderValueType::Vec3Float32 => 12,
      PrimitiveShaderValueType::Vec4Float32 => 16,
      PrimitiveShaderValueType::Mat2Float32 => 16,
      PrimitiveShaderValueType::Mat3Float32 => 36,
      PrimitiveShaderValueType::Mat4Float32 => 64,
      PrimitiveShaderValueType::Vec2Uint32 => 8,
      PrimitiveShaderValueType::Vec3Uint32 => 12,
      PrimitiveShaderValueType::Vec4Uint32 => 16,
    }
  }
}
