use crate::*;

/// The memory layout rules used to compute the offset and size of the shader types.
///
/// For the uniform buffer, the host shareable struct always carries its host layout (see
/// [ShaderStructHostLayout]), and the shader side follows it directly, so there is no dedicated
/// std140 rule here.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StructLayoutTarget {
  /// The natural WGSL layout, which is the storage address space layout. The layout is fully
  /// determined by the type itself. If the struct has a host layout, the host layout is used.
  Std430,
  /// All types are 4 bytes aligned and no padding exists, only used in shader side u32
  /// serialization, the host layout is ignored.
  Packed,
}

/// The layout of the struct fields.
pub struct StructFieldsLayout {
  /// the start byte offset of each field
  pub offsets: Vec<usize>,
  /// the byte offset right after the last field
  pub end: usize,
  pub alignment: usize,
  /// the end rounded up to the alignment
  pub size: usize,
}

/// Compute the fields layout by layout rules, the host layout is not considered at this level,
/// see [ShaderStructMetaInfo::fields_layout].
pub fn struct_fields_layout_by_rule(
  fields: &[ShaderStructFieldMetaInfo],
  target: StructLayoutTarget,
) -> StructFieldsLayout {
  let mut offsets = Vec::with_capacity(fields.len());
  let mut end = 0;
  let mut alignment = match target {
    StructLayoutTarget::Std430 => 1,
    StructLayoutTarget::Packed => 4,
  };
  for field in fields {
    let field_align = field.ty.align_of_self(target);
    let offset = round_up(field_align, end);
    offsets.push(offset);
    end = offset + field.ty.size_of_self(target);
    alignment = alignment.max(field_align);
  }
  StructFieldsLayout {
    offsets,
    end,
    alignment,
    size: round_up(alignment, end),
  }
}

impl ShaderStructMetaInfo {
  pub fn fields_layout(&self, target: StructLayoutTarget) -> StructFieldsLayout {
    let by_rule = struct_fields_layout_by_rule(&self.fields, target);
    match (&self.host_layout, target) {
      (Some(host_layout), StructLayoutTarget::Std430) => {
        // the natural alignment is not affected by the host layout, the shader backend only
        // insert u32 paddings to follow the host layout.
        StructFieldsLayout {
          offsets: host_layout.field_offsets.clone(),
          end: host_layout.size,
          alignment: by_rule.alignment,
          size: host_layout.size,
        }
      }
      _ => by_rule,
    }
  }

  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    struct_fields_layout_by_rule(&self.fields, target).alignment
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    match (&self.host_layout, target) {
      (Some(host_layout), StructLayoutTarget::Std430) => host_layout.size,
      _ => struct_fields_layout_by_rule(&self.fields, target).size,
    }
  }
}

impl ShaderUnSizedStructMetaInfo {
  /// return (the byte offset of the runtime sized array field, the struct alignment)
  pub fn runtime_array_layout(&self, target: StructLayoutTarget) -> (usize, usize) {
    let sized = struct_fields_layout_by_rule(&self.sized_fields, target);
    let array_ty = &self.last_dynamic_array_field.1;
    let array_align = array_align_of_element(array_ty, target);
    (
      round_up(array_align, sized.end),
      sized.alignment.max(array_align),
    )
  }
}

impl ShaderValueSingleType {
  /// The minimal byte size required for the buffer binding of this type. For the runtime sized
  /// array, it is assumed to have one element, which is same as the WebGPU spec.
  pub fn min_binding_size(&self) -> Option<core::num::NonZeroU64> {
    let layout = StructLayoutTarget::Std430;
    let size = match self {
      ShaderValueSingleType::Sized(ty) => ty.size_of_self(layout),
      ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) => {
        array_stride_of_element(ty, layout)
      }
      ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedStruct(meta)) => {
        let (array_offset, alignment) = meta.runtime_array_layout(layout);
        let stride = array_stride_of_element(&meta.last_dynamic_array_field.1, layout);
        round_up(alignment, array_offset + stride)
      }
      _ => return None,
    };
    core::num::NonZeroU64::new(size as u64)
  }
}

/// Round `n` up to the nearest alignment boundary.
pub const fn round_up(k: usize, n: usize) -> usize {
  // equivalent to:
  // match n % k {
  //     0 => n,
  //     rem => n + (k - rem),
  // }
  let mask = k - 1;
  (n + mask) & !mask
}

/// The alignment of an array(fixed size or runtime sized) whose element type is `element`
pub fn array_align_of_element(element: &ShaderSizedValueType, target: StructLayoutTarget) -> usize {
  element.align_of_self(target)
}

/// The byte distance between adjacent elements of an array(fixed size or runtime sized) whose
/// element type is `element`.
pub fn array_stride_of_element(
  element: &ShaderSizedValueType,
  target: StructLayoutTarget,
) -> usize {
  round_up(
    array_align_of_element(element, target),
    element.size_of_self(target),
  )
}

impl ShaderSizedValueType {
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      ShaderSizedValueType::Atomic(t) => t.align_of_self(),
      ShaderSizedValueType::Primitive(t) => t.align_of_self(target),
      ShaderSizedValueType::Struct(t) => t.align_of_self(target),
      ShaderSizedValueType::FixedSizeArray(t, _) => array_align_of_element(t, target),
    }
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      ShaderSizedValueType::Atomic(t) => t.size_of_self(),
      ShaderSizedValueType::Primitive(t) => t.size_of_self(target),
      ShaderSizedValueType::Struct(t) => t.size_of_self(target),
      ShaderSizedValueType::FixedSizeArray(ty, size) => size * array_stride_of_element(ty, target),
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
  pub fn align_of_self(&self, target: StructLayoutTarget) -> usize {
    if target == StructLayoutTarget::Packed {
      return 4;
    }
    match self {
      PrimitiveShaderValueType::Scalar(_) => 4,
      PrimitiveShaderValueType::Vector {
        size: VectorSize::Bi,
        ..
      } => 8,
      PrimitiveShaderValueType::Vector { .. } => 16,
      PrimitiveShaderValueType::Matrix { rows, .. } => match rows {
        VectorSize::Bi => 8,
        VectorSize::Tri | VectorSize::Quad => 16,
      },
    }
  }

  pub fn size_of_self(&self, target: StructLayoutTarget) -> usize {
    match self {
      PrimitiveShaderValueType::Scalar(_) => 4,
      PrimitiveShaderValueType::Vector { size, .. } => 4 * *size as usize,
      PrimitiveShaderValueType::Matrix { columns, rows, .. } => {
        *columns as usize * matrix_column_stride(*rows, target)
      }
    }
  }
}

/// The byte distance between adjacent columns of a matrix, the matrix is treated as an array of
/// column vectors, so for example the column of mat3x3 is padded to 16 bytes (except packed).
pub fn matrix_column_stride(rows: VectorSize, target: StructLayoutTarget) -> usize {
  let column_size = 4 * rows as usize;
  if target == StructLayoutTarget::Packed {
    return column_size;
  }
  let column_align = match rows {
    VectorSize::Bi => 8,
    VectorSize::Tri | VectorSize::Quad => 16,
  };
  round_up(column_align, column_size)
}
