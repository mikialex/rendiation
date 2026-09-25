use crate::*;

impl PrimitiveShaderValueType {
  pub fn u32_count_of_self(&self, layout: StructLayoutTarget) -> usize {
    self.size_of_self(layout) / 4
  }

  pub fn is_single_primitive(&self) -> bool {
    self.u32_count_of_self(StructLayoutTarget::Packed) == 1
  }

  /// returns (column stride in u32 count, column type)
  /// calculate column count by u32_count / column_stride
  pub fn mat_row_info(&self, target: StructLayoutTarget) -> Option<(usize, ShaderSizedValueType)> {
    match self {
      PrimitiveShaderValueType::Matrix { rows, scalar, .. } => Some((
        matrix_column_stride(*rows, target) / 4,
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vector(*rows, *scalar)),
      )),
      _ => None,
    }
  }
}

impl ShaderSizedValueType {
  pub fn u32_size_count(&self, layout: StructLayoutTarget) -> u32 {
    self.size_of_self(layout) as u32 / 4
  }

  pub fn load_from_u32_buffer(
    &self,
    target: &ShaderReadonlyPtrOf<[u32]>,
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
            convert_to: p.scalar(),
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
        let parameters = f
          .fields
          .iter()
          .zip(f.fields_layout(layout).offsets)
          .map(|(fty, f_offset)| {
            let offset = offset + val(f_offset as u32 / 4);
            fty.ty.load_from_u32_buffer(target, offset, layout)
          })
          .collect();

        ShaderNodeExpr::Compose {
          target: self.clone(),
          parameters,
        }
        .insert_api_raw()
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let mut offset = offset;
        let stride = val(array_stride_of_element(ty, layout) as u32 / 4);
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
            convert_to: ScalarType::U32,
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
        let offsets = f.fields_layout(layout).offsets;
        for (i, (fty, f_offset)) in f.fields.iter().zip(offsets).enumerate() {
          fty.ty.store_into_u32_buffer(
            unsafe { index_access_field(source, i) },
            target,
            offset + val(f_offset as u32 / 4),
            layout,
          );
        }
      }
      ShaderSizedValueType::FixedSizeArray(ty, size) => {
        let stride = val(array_stride_of_element(ty, layout) as u32 / 4);
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
    target: &ShaderReadonlyPtrOf<[u32]>,
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
