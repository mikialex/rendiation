use core::num::NonZeroU32;

use crate::*;

impl ShaderAPINagaImpl {
  pub(crate) fn register_sized_ty(&mut self, ty: ShaderSizedValueType) -> naga::Handle<naga::Type> {
    self.register_ty_impl(ShaderValueType::Single(ShaderValueSingleType::Sized(ty)))
  }

  pub(crate) fn register_primitive_ty(
    &mut self,
    ty: PrimitiveShaderValueType,
  ) -> naga::Handle<naga::Type> {
    self.register_sized_ty(ShaderSizedValueType::Primitive(ty))
  }

  pub(crate) fn register_ty_impl(&mut self, ty: ShaderValueType) -> naga::Handle<naga::Type> {
    if let Some(handle) = self.ty_mapping.get(&ty) {
      return *handle;
    }

    let mut name = None;
    let mut padded_struct_member_fields = None;

    let naga_ty = match &ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Atomic(t) => naga::TypeInner::Atomic(map_atomic_scalar(*t)),
          ShaderSizedValueType::Primitive(p) => map_primitive_type(*p),
          ShaderSizedValueType::Struct(st) => {
            name = st.name.to_owned().into();
            let (inner, member_fields) = gen_struct_define(self, st);
            if member_fields.iter().any(|f| f.is_none()) {
              padded_struct_member_fields = Some(member_fields);
            }
            inner
          }
          ShaderSizedValueType::FixedSizeArray(ty, size) => {
            let base = self.register_sized_ty(*ty.clone());
            naga::TypeInner::Array {
              base,
              size: naga::ArraySize::Constant(NonZeroU32::new(*size as u32).unwrap()),
              stride: self.natural_layout(base).to_stride(),
            }
          }
        },
        ShaderValueSingleType::Unsized(ty) => match ty {
          ShaderUnSizedValueType::UnsizedArray(ty) => {
            let base = self.register_sized_ty(*ty.clone());
            naga::TypeInner::Array {
              base,
              size: naga::ArraySize::Dynamic,
              stride: self.natural_layout(base).to_stride(),
            }
          }
          ShaderUnSizedValueType::UnsizedStruct(meta) => {
            name = meta.name.to_owned().into();
            gen_unsized_struct_define(self, meta)
          }
        },
        ShaderValueSingleType::Sampler(sampler) => naga::TypeInner::Sampler {
          comparison: matches!(sampler, SamplerBindingType::Comparison),
        },
        ShaderValueSingleType::Texture {
          dimension,
          sample_type,
          multi_sampled,
        } => {
          let (dim, arrayed) = map_image_dimension(*dimension);
          let class = map_texture_sample_type(*sample_type, *multi_sampled);
          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
        ShaderValueSingleType::StorageTexture {
          dimension,
          format,
          access,
        } => {
          if matches!(
            dimension,
            TextureViewDimension::Cube | TextureViewDimension::CubeArray
          ) {
            panic!("Unsupported storage texture dimension");
          }
          let (dim, arrayed) = map_image_dimension(*dimension);
          let format = map_storage_format(*format);
          let access = map_storage_access(*access);

          let class = naga::ImageClass::Storage { format, access };

          naga::TypeInner::Image {
            dim,
            arrayed,
            class,
          }
        }
        &ShaderValueSingleType::AccelerationStructure => naga::TypeInner::AccelerationStructure {
          vertex_return: true,
        },
        &ShaderValueSingleType::RayQuery => naga::TypeInner::RayQuery {
          vertex_return: true,
        },
      },
      ShaderValueType::BindingArray { count, ty } => naga::TypeInner::BindingArray {
        base: self.register_ty_impl(ShaderValueType::Single(ty.clone())),
        size: naga::ArraySize::Constant(NonZeroU32::new(*count as u32).unwrap()),
      },
      ShaderValueType::Never => unreachable!(),
    };
    let naga_ty = naga::Type {
      name,
      inner: naga_ty,
    };
    let type_handle = self.module.types.insert(naga_ty, Span::UNDEFINED);
    self.ty_mapping.insert(ty, type_handle);
    if let Some(member_fields) = padded_struct_member_fields {
      self.padded_structs.insert(type_handle, member_fields);
    }
    type_handle
  }

  /// The natural WGSL layout of the type, which only depends on the type itself.
  fn natural_layout(&mut self, ty: naga::Handle<naga::Type>) -> naga::proc::TypeLayout {
    self
      .layouter
      .update(self.module.to_ctx())
      .expect("failed to compute the naga type layout");
    self.layouter[ty]
  }

  /// Insert zero values for the padding members if the struct has explicit padding members.
  pub(crate) fn fill_struct_padding_components(
    &mut self,
    ty: naga::Handle<naga::Type>,
    components: Vec<naga::Handle<naga::Expression>>,
    append: fn(&mut Self, naga::Expression) -> naga::Handle<naga::Expression>,
  ) -> Vec<naga::Handle<naga::Expression>> {
    let Some(member_fields) = self.padded_structs.get(&ty).cloned() else {
      return components;
    };
    member_fields
      .iter()
      .map(|field| match field {
        Some(field_index) => components[*field_index],
        None => append(self, naga::Expression::Literal(naga::Literal::U32(0))),
      })
      .collect()
  }

  /// Map the struct field index into the naga struct member index, they are different when the
  /// struct has explicit padding members.
  pub(crate) fn map_struct_field_index(
    &mut self,
    base: naga::Handle<naga::Expression>,
    field_index: usize,
  ) -> u32 {
    if self.padded_structs.is_empty() {
      return field_index as u32;
    }

    let struct_ty = match self.resolve_expr_type(base) {
      naga::proc::TypeResolution::Handle(ty) => match self.module.types[ty].inner {
        naga::TypeInner::Pointer { base, .. } => base,
        _ => ty,
      },
      naga::proc::TypeResolution::Value(naga::TypeInner::Pointer { base, .. }) => base,
      _ => return field_index as u32,
    };

    match self.padded_structs.get(&struct_ty) {
      Some(member_fields) => member_fields
        .iter()
        .position(|f| *f == Some(field_index))
        .expect("struct field index out of bound") as u32,
      None => field_index as u32,
    }
  }
}

/// The result of building naga struct members, see [build_struct_members]
struct StructMembers {
  members: Vec<naga::StructMember>,
  /// map each member to the field index, None means it's an explicit padding member
  member_fields: Vec<Option<usize>>,
  /// the byte offset right after the last member
  end_offset: u32,
  alignment: naga::proc::Alignment,
}

impl StructMembers {
  /// Append u32 padding members until the end offset reaches the target offset.
  fn pad_to(&mut self, api: &mut ShaderAPINagaImpl, target: u32) {
    assert!(target >= self.end_offset);
    // all shader types' size are multiple of 4 bytes, so the gap is always able to be filled
    assert!((target - self.end_offset).is_multiple_of(4));
    let u32_ty = api.register_primitive_ty(PrimitiveShaderValueType::u32());
    while self.end_offset < target {
      let padding_index = self.member_fields.iter().filter(|f| f.is_none()).count();
      self.members.push(naga::StructMember {
        name: format!("padding_{padding_index}").into(),
        ty: u32_ty,
        binding: None,
        offset: self.end_offset,
      });
      self.member_fields.push(None);
      self.end_offset += 4;
    }
  }
}

/// Build the struct members in the natural WGSL layout, which means the layout is fully
/// determined by the member types, without any explicit offset or size attribute.
///
/// This is required because some backends ignore the member offsets and span in naga IR and
/// recompute the layout from the member types, for example the WGSL text output (used by the
/// browser WebGPU implementation) and the GLSL output. Any layout that is not natural will
/// silently change in these backends.
///
/// For host shareable struct, the host layout is the source of truth. If the host offset is
/// larger than the natural one (for example std140 requires the nested struct aligned to 16),
/// explicit u32 padding members are inserted to make it natural.
fn build_struct_members(
  api: &mut ShaderAPINagaImpl,
  struct_name: &str,
  fields: &[ShaderStructFieldMetaInfo],
  host_layout: Option<&ShaderStructHostLayout>,
) -> StructMembers {
  let mut result = StructMembers {
    members: Vec::with_capacity(fields.len()),
    member_fields: Vec::with_capacity(fields.len()),
    end_offset: 0,
    alignment: naga::proc::Alignment::ONE,
  };

  for (index, field) in fields.iter().enumerate() {
    let ty = api.register_sized_ty(field.ty.clone());
    let layout = api.natural_layout(ty);
    result.alignment = result.alignment.max(layout.alignment);
    let natural_offset = layout.alignment.round_up(result.end_offset);

    let offset = if let Some(host_layout) = host_layout {
      let host_offset = host_layout.field_offsets[index] as u32;
      assert!(
        host_offset >= natural_offset && layout.alignment.is_aligned(host_offset),
        "the {:?} host layout of struct `{struct_name}` field `{}` is invalid for WGSL, \
         host offset: {host_offset}, natural offset: {natural_offset}",
        host_layout.target,
        field.name,
      );
      if host_offset != natural_offset {
        result.pad_to(api, host_offset);
      }
      host_offset
    } else {
      natural_offset
    };

    let binding = field.ty_deco.map(|deco| match deco {
      ShaderFieldDecorator::BuiltIn(bt) => naga::Binding::BuiltIn(map_built_in(bt)),
      ShaderFieldDecorator::Location(location, interpolation) => naga::Binding::Location {
        location: location as u32,
        interpolation: interpolation.map(map_interpolation),
        sampling: None,
        blend_src: None,
        per_primitive: false,
      },
    });

    result.members.push(naga::StructMember {
      name: field.name.clone().into(),
      ty,
      binding,
      offset,
    });
    result.member_fields.push(Some(index));
    result.end_offset = offset + layout.size;
  }

  result
}

/// return the struct type and the member to field index mapping
pub(crate) fn gen_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: &ShaderStructMetaInfo,
) -> (naga::TypeInner, Vec<Option<usize>>) {
  let mut members = build_struct_members(api, &meta.name, &meta.fields, meta.host_layout.as_ref());
  assert!(!members.members.is_empty());

  let natural_span = members.alignment.round_up(members.end_offset);
  let span = if let Some(host_layout) = &meta.host_layout {
    let host_size = host_layout.size as u32;
    assert!(
      host_size >= natural_span && members.alignment.is_aligned(host_size),
      "the {:?} host layout of struct `{}` is invalid for WGSL, \
       host size: {host_size}, natural size: {natural_span}",
      host_layout.target,
      meta.name,
    );
    members.pad_to(api, host_size);
    host_size
  } else {
    natural_span
  };

  let inner = naga::TypeInner::Struct {
    members: members.members,
    span,
  };
  (inner, members.member_fields)
}

fn gen_unsized_struct_define(
  api: &mut ShaderAPINagaImpl,
  meta: &ShaderUnSizedStructMetaInfo,
) -> naga::TypeInner {
  let mut members = build_struct_members(api, &meta.name, &meta.sized_fields, None);

  let (name, array_ty) = &meta.last_dynamic_array_field;
  let ty = api.register_ty_impl(ShaderValueType::Single(ShaderValueSingleType::Unsized(
    ShaderUnSizedValueType::UnsizedArray(Box::new(*array_ty.clone())),
  )));
  // the size of runtime sized array is treated as its stride
  let layout = api.natural_layout(ty);
  let offset = layout.alignment.round_up(members.end_offset);
  let alignment = members.alignment.max(layout.alignment);

  members.members.push(naga::StructMember {
    name: name.to_string().into(),
    ty,
    binding: None,
    offset,
  });

  naga::TypeInner::Struct {
    members: members.members,
    span: alignment.round_up(offset + layout.size),
  }
}
