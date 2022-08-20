use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum StructLayoutTarget {
  Std140,
  Std430,
}

impl ShaderStructMetaInfoOwned {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    self
      .fields
      .iter()
      .map(|field| field.ty.align_of_self(target))
      .max()
      .unwrap_or(1)
  }

  pub fn size_of_self(&self) -> usize {
    todo!()
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
      ShaderStructMemberValueType::Struct(t) => {
        let align = (*t).to_owned().align_of_self(target);
        match target {
          StructLayoutTarget::Std140 => round_up(16, align),
          StructLayoutTarget::Std430 => align,
        }
      }
      ShaderStructMemberValueType::FixedSizeArray((t, _)) => {
        let align = t.align_of_self(target);
        match target {
          StructLayoutTarget::Std140 => round_up(16, align),
          StructLayoutTarget::Std430 => align,
        }
      }
    }
  }

  pub fn size_of_self(&self) -> usize {
    match self {
      ShaderStructMemberValueType::Primitive(t) => t.size_of_self(),
      ShaderStructMemberValueType::Struct(t) => (*t).to_owned().size_of_self(),
      ShaderStructMemberValueType::FixedSizeArray(t) => todo!(),
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
    }
  }

  pub fn align_of_self_std140(&self) -> usize {
    self.align_of_self()
  }

  pub fn size_of_self(&self) -> usize {
    match self {
      PrimitiveShaderValueType::Bool => 4,
      PrimitiveShaderValueType::Int32 => 4,
      PrimitiveShaderValueType::Uint32 => 4,
      PrimitiveShaderValueType::Float32 => 4,
      PrimitiveShaderValueType::Vec2Float32 => 8,
      PrimitiveShaderValueType::Vec3Float32 => 16,
      PrimitiveShaderValueType::Vec4Float32 => 16,
      PrimitiveShaderValueType::Mat2Float32 => 16,
      PrimitiveShaderValueType::Mat3Float32 => 64,
      PrimitiveShaderValueType::Mat4Float32 => 64,
    }
  }
}
