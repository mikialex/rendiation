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
      PrimitiveShaderValueType::Mat4x3Float32 => ValueKind::Float,
    }
  }

  pub fn u32_count_of_self(&self, layout: StructLayoutTarget) -> usize {
    let is_packed = matches!(layout, StructLayoutTarget::Packed);
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
      PrimitiveShaderValueType::Mat3Float32 => {
        if is_packed {
          9
        } else {
          16
        }
      }
      PrimitiveShaderValueType::Mat4Float32 => 16,
      PrimitiveShaderValueType::Mat4x3Float32 => 12,
    }
  }

  pub fn is_single_primitive(&self) -> bool {
    self.u32_count_of_self(StructLayoutTarget::Packed) == 1
  }

  /// returns (row stride, row type)
  /// calculate row count by f32_count / row_stride
  pub fn mat_row_info(&self, target: StructLayoutTarget) -> Option<(usize, ShaderSizedValueType)> {
    match self {
      PrimitiveShaderValueType::Mat2Float32 => (
        2,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec2Float32),
      ),
      PrimitiveShaderValueType::Mat3Float32 => (
        if target == StructLayoutTarget::Packed {
          3
        } else {
          4
        },
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec3Float32),
      ),
      PrimitiveShaderValueType::Mat4Float32 => (
        4,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec4Float32),
      ),
      PrimitiveShaderValueType::Mat4x3Float32 => (
        3,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Vec3Float32),
      ),
      _ => return None,
    }
    .into()
  }
}

impl ShaderSizedValueType {
  pub fn u32_size_count(&self, layout: StructLayoutTarget) -> u32 {
    self.size_of_self(layout) as u32 / 4
  }

  pub fn load_from_u32_buffer(
    &self,
    target: &ShaderPtrOf<[u32]>,
    mut offset: Node<u32>,
    layout: StructLayoutTarget,
  ) -> ShaderNodeRawHandle {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to load from buffer"),
      ShaderSizedValueType::Primitive(p) => {
        let size = ShaderSizedValueType::Primitive(*p).u32_size_count(layout);
        let mut parameters = Vec::new();
        for _ in 0..size {
          let u32_read = target.index(offset).load();
          offset += val(1);
          let handle = ShaderNodeExpr::Convert {
            source: u32_read.handle(),
            convert_to: p.channel_ty(),
            convert: None,
          }
          .insert_api_raw();
          parameters.push(handle);
        }

        if let Some((row_stride, row_ty)) = p.mat_row_info(layout) {
          let row_size = row_ty.u32_size_count(layout);
          let mut parameter_row = Vec::with_capacity(row_stride);
          for sub_parameters in parameters.chunks_exact(row_stride) {
            let sub_parameters = sub_parameters[0..row_size as usize].to_vec();
            parameter_row.push(
              ShaderNodeExpr::Compose {
                target: row_ty.clone(),
                parameters: sub_parameters,
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
      ShaderSizedValueType::Struct(f) => {
        let offset = offset;
        let mut parameters = Vec::new();
        let tail_pad = iter_field_start_offset_in_bytes(&f.fields, layout, &mut |f_offset, fty| {
          let offset = offset + val(f_offset as u32 / 4);
          parameters.push(fty.ty.load_from_u32_buffer(target, offset, layout));
        });

        if let Some(TailPaddingInfo {
          pad_size_in_bytes, ..
        }) = tail_pad
        {
          let pad_count = pad_size_in_bytes / 4;
          // not using array here because I do not want hit anther strange layout issue!
          for _ in 0..pad_count {
            parameters.push(val(0_u32).handle());
          }
        }

        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api_raw()
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let mut offset = offset;
        let stride = val(ty.u32_size_count(layout));
        let mut parameters = Vec::new();
        for _ in 0..*size {
          parameters.push(ty.load_from_u32_buffer(target, offset, layout));
          offset += stride;
        }
        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api_raw()
      }
    }
  }

  pub fn store_into_u32_buffer(
    &self,
    source: ShaderNodeRawHandle,
    target: &ShaderPtrOf<[u32]>,
    mut offset: Node<u32>,
    layout: StructLayoutTarget,
  ) {
    match self {
      ShaderSizedValueType::Atomic(_) => unreachable!("atomic is not able to store into buffer"),
      ShaderSizedValueType::Primitive(p) => {
        fn index_and_write(
          target: &ShaderPtrOf<[u32]>,
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

        if let Some((row_stride, row_ty)) = p.mat_row_info(layout) {
          // all matrix types are f32, f32 count === u32 count
          let f32_size = ShaderSizedValueType::Primitive(*p).u32_size_count(layout);
          let row_stride = row_stride as u32;
          let row_count = f32_size / row_stride;
          let row_pack_size = row_ty.u32_size_count(layout);

          for i in 0..row_count {
            let row = unsafe { index_access_field(source, i as usize) };
            for j in 0..row_pack_size {
              index_and_write(target, offset, row, Some(j));
              offset += val(1);
            }
            if row_stride - row_pack_size > 0 {
              offset += val(row_stride - row_pack_size);
            }
          }
        } else {
          for i in 0..ShaderSizedValueType::Primitive(*p).u32_size_count(layout) {
            let single = p.is_single_primitive();
            index_and_write(target, offset, source, (!single).then_some(i));
            offset += val(1);
          }
        }
      }
      ShaderSizedValueType::Struct(f) => {
        let mut i = 0;
        iter_field_start_offset_in_bytes(&f.fields, layout, &mut |f_offset, fty| {
          fty.ty.store_into_u32_buffer(
            unsafe { index_access_field(source, i) },
            target,
            offset + val(f_offset as u32 / 4),
            layout,
          );
          i += 1;
        });
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let stride = val(ty.u32_size_count(layout));
        for i in 0..*size {
          ty.store_into_u32_buffer(
            unsafe { index_access_field(source, i) },
            target,
            offset,
            layout,
          );
          offset += stride;
        }
      }
    }
  }
}

impl<T: ShaderSizedValueNodeType> Node<T> {
  pub fn u32_size_count(layout: StructLayoutTarget) -> u32 {
    T::sized_ty().u32_size_count(layout)
  }

  pub fn load_from_u32_buffer(
    target: &ShaderPtrOf<[u32]>,
    offset: Node<u32>,
    layout: StructLayoutTarget,
  ) -> Self {
    unsafe {
      T::sized_ty()
        .load_from_u32_buffer(target, offset, layout)
        .into_node()
    }
  }

  pub fn store_into_u32_buffer(
    self,
    target: &ShaderPtrOf<[u32]>,
    offset: Node<u32>,
    layout: StructLayoutTarget,
  ) {
    T::sized_ty().store_into_u32_buffer(self.handle(), target, offset, layout)
  }
}
