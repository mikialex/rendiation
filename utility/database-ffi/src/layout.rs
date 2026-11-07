use crate::*;

/// The C side representation of a facet shape
#[derive(Clone)]
pub enum CType {
  Primitive {
    kind: CPrimitive,
    shape: &'static Shape,
  },
  EntityHandle,
  Array {
    element: Box<CType>,
    count: usize,
  },
  Struct(Arc<CStruct>),
  /// `{ bool has_value; T value; }`
  Optional(Box<CType>),
  Enum(Arc<CEnum>),
  /// `String` as `{ const char *ptr; size_t len; }`, the read value is always nul terminated.
  Str,
  /// `Vec<T>` as `{ const T *ptr; size_t len; }`
  Slice {
    shape: &'static Shape,
    element: Box<CType>,
    /// the element has the same layout in rust and C, and any bit pattern is valid, so the
    /// elements can be copied in bulk.
    bulk_copy: bool,
  },
  /// A transparent wrapper (for example `NonZero<T>`), represented as the inner type in C.
  Inner {
    shape: &'static Shape,
    inner: Box<CType>,
  },
  /// A smart pointer (for example `Arc<T>`), represented as the pointee value in C.
  Pointer {
    pointee: Box<CType>,
  },
  /// A type converted through a facet proxy (`#[facet(proxy = P)]`), represented as the proxy in C.
  Proxy {
    def: &'static facet::ProxyDef,
    field_level: bool,
    proxy: Box<CType>,
  },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CPrimitive {
  Bool,
  Int { bytes: usize, signed: bool },
  Float { bytes: usize },
}

pub struct CStruct {
  pub c_name: String,
  pub fields: Vec<CField>,
  /// zero sized rust fields, they are not visible in C and are filled by default when building.
  pub zst_fields: Vec<usize>,
  pub size: usize,
  pub align: usize,
}

pub struct CField {
  pub c_name: String,
  pub rust_index: usize,
  pub rust_offset: usize,
  pub ty: CType,
  pub offset: usize,
}

/// Field-less enums are mapped to `enum class : uint32_t`, others are mapped to a
/// `{ uint32_t tag; union { ... } payload; }` struct.
pub struct CEnum {
  pub c_name: String,
  pub variants: Vec<CVariant>,
  pub payload_offset: usize,
  pub payload_size: usize,
  pub size: usize,
  pub align: usize,
}

pub struct CVariant {
  pub c_name: String,
  pub tag: u32,
  pub payload: Option<Arc<CStruct>>,
}

impl CEnum {
  pub fn has_payload(&self) -> bool {
    self.variants.iter().any(|v| v.payload.is_some())
  }
}

/// the layout of [CType::Str] and [CType::Slice]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CSlice {
  pub ptr: *const u8,
  pub len: usize,
}

impl CType {
  pub fn size(&self) -> usize {
    match self.c_visible() {
      CType::Primitive { kind, .. } => kind.size(),
      CType::EntityHandle => size_of::<FfiEntityHandle>(),
      CType::Array { element, count } => element.size() * count,
      CType::Struct(s) => s.size,
      CType::Optional(inner) => {
        align_up(optional_value_offset(inner) + inner.size(), inner.align())
      }
      CType::Enum(e) => e.size,
      CType::Str | CType::Slice { .. } => size_of::<CSlice>(),
      _ => unreachable!(),
    }
  }

  pub fn align(&self) -> usize {
    match self.c_visible() {
      CType::Primitive { kind, .. } => kind.size(),
      CType::EntityHandle => align_of::<FfiEntityHandle>(),
      CType::Array { element, .. } => element.align(),
      CType::Struct(s) => s.align,
      CType::Optional(inner) => inner.align(),
      CType::Enum(e) => e.align,
      CType::Str | CType::Slice { .. } => align_of::<CSlice>(),
      _ => unreachable!(),
    }
  }

  /// Unwrap the wrappers that are invisible to C, the C side only sees the innermost type.
  pub fn c_visible(&self) -> &CType {
    match self {
      CType::Inner { inner, .. } => inner.c_visible(),
      CType::Pointer { pointee } => pointee.c_visible(),
      CType::Proxy { proxy, .. } => proxy.c_visible(),
      other => other,
    }
  }

  /// If the C value contains pointers to the memory owned by the ffi side.
  pub fn has_heap_data(&self) -> bool {
    match self.c_visible() {
      CType::Str | CType::Slice { .. } => true,
      CType::Array { element, .. } => element.has_heap_data(),
      CType::Struct(s) => s.fields.iter().any(|f| f.ty.has_heap_data()),
      CType::Optional(inner) => inner.has_heap_data(),
      CType::Enum(e) => e
        .variants
        .iter()
        .filter_map(|v| v.payload.as_ref())
        .any(|p| p.fields.iter().any(|f| f.ty.has_heap_data())),
      _ => false,
    }
  }

  /// A canonical description of the C layout, used to compute the schema hash.
  pub fn write_signature(&self, out: &mut String) {
    match self.c_visible() {
      CType::Primitive { kind, .. } => out.push_str(kind.c_name()),
      CType::EntityHandle => out.push('H'),
      CType::Array { element, count } => {
        element.write_signature(out);
        write!(out, "[{count}]").unwrap();
      }
      CType::Struct(s) => {
        write!(out, "{}{{", s.c_name).unwrap();
        for f in &s.fields {
          write!(out, "{}@{}:", f.c_name, f.offset).unwrap();
          f.ty.write_signature(out);
          out.push(',');
        }
        out.push('}');
      }
      CType::Optional(inner) => {
        out.push('?');
        inner.write_signature(out);
      }
      CType::Enum(e) => {
        write!(out, "{}<", e.c_name).unwrap();
        for v in &e.variants {
          write!(out, "{}={}", v.c_name, v.tag).unwrap();
          if let Some(payload) = &v.payload {
            CType::Struct(payload.clone()).write_signature(out);
          }
          out.push(',');
        }
        out.push('>');
      }
      CType::Str => out.push('s'),
      CType::Slice { element, .. } => {
        out.push('S');
        element.write_signature(out);
      }
      _ => unreachable!(),
    }
  }
}

impl CPrimitive {
  pub fn size(&self) -> usize {
    match self {
      CPrimitive::Bool => 1,
      CPrimitive::Int { bytes, .. } => *bytes,
      CPrimitive::Float { bytes } => *bytes,
    }
  }

  pub fn c_name(&self) -> &'static str {
    match self {
      CPrimitive::Bool => "bool",
      CPrimitive::Int { bytes: 1, signed } => {
        if *signed {
          "int8_t"
        } else {
          "uint8_t"
        }
      }
      CPrimitive::Int { bytes: 2, signed } => {
        if *signed {
          "int16_t"
        } else {
          "uint16_t"
        }
      }
      CPrimitive::Int { bytes: 4, signed } => {
        if *signed {
          "int32_t"
        } else {
          "uint32_t"
        }
      }
      CPrimitive::Int { signed, .. } => {
        if *signed {
          "int64_t"
        } else {
          "uint64_t"
        }
      }
      CPrimitive::Float { bytes: 4 } => "float",
      CPrimitive::Float { .. } => "double",
    }
  }
}

pub(crate) fn optional_value_offset(inner: &CType) -> usize {
  align_up(1, inner.align())
}

fn align_up(v: usize, align: usize) -> usize {
  v.div_ceil(align) * align
}

/// Build [CType] for shapes. The struct and enum are cached by shape so that the same rust type
/// always maps to the same C type, and the C names are deduplicated in the builder scope.
#[derive(Default)]
pub struct CTypeBuilder {
  cache: FastHashMap<facet::ConstTypeId, CType>,
  used_names: FastHashMap<String, facet::ConstTypeId>,
}

/// The arrays are built by facet-reflect frame by frame, which has a size limit.
const MAX_ARRAY_LEN: usize = 63;

impl CTypeBuilder {
  pub fn build(&mut self, shape: &'static Shape) -> Result<CType, FfiError> {
    if let Some(ty) = self.cache.get(&shape.id) {
      return Ok(ty.clone());
    }
    let ty = self.build_impl(shape)?;
    if matches!(ty, CType::Struct(_) | CType::Enum(_)) {
      self.cache.insert(shape.id, ty.clone());
    }
    Ok(ty)
  }

  fn build_impl(&mut self, shape: &'static Shape) -> Result<CType, FfiError> {
    if shape.is_shape(RawEntityHandle::SHAPE) {
      return Ok(CType::EntityHandle);
    }
    if shape.is_shape(String::SHAPE) {
      return Ok(CType::Str);
    }
    if let Some(def) = shape.proxy {
      return self.build_proxy(def, false);
    }

    match shape.def {
      Def::Option(o) => return Ok(CType::Optional(Box::new(self.build(o.t())?))),
      Def::Array(a) => {
        if a.n > MAX_ARRAY_LEN {
          return Err(FfiError(format!(
            "array longer than {MAX_ARRAY_LEN} is not supported"
          )));
        }
        let element = Box::new(self.build(a.t())?);
        return Ok(CType::Array {
          element,
          count: a.n,
        });
      }
      Def::List(l) => {
        let element = self.build(l.t())?;
        return Ok(CType::Slice {
          shape,
          bulk_copy: bulk_copy_compatible(&element, l.t()),
          element: Box::new(element),
        });
      }
      Def::Pointer(p) => {
        let pointee = p
          .pointee()
          .ok_or_else(|| FfiError(format!("{} has no pointee", shape.type_identifier)))?;
        if !matches!(
          p.known,
          Some(facet::KnownPointer::Arc | facet::KnownPointer::Box | facet::KnownPointer::Rc)
        ) {
          return Err(FfiError(format!(
            "pointer {} is not supported",
            shape.type_identifier
          )));
        }
        let pointee = Box::new(self.build(pointee)?);
        return Ok(CType::Pointer { pointee });
      }
      Def::Scalar | Def::Undefined => {}
      _ => {
        return Err(FfiError(format!(
          "{} is not supported (map or set like container)",
          shape.type_identifier
        )));
      }
    }

    if let Some(inner) = shape.inner {
      let inner = Box::new(self.build(inner)?);
      return Ok(CType::Inner { shape, inner });
    }

    match shape.ty {
      Type::Primitive(PrimitiveType::Boolean) => Ok(CType::Primitive {
        kind: CPrimitive::Bool,
        shape,
      }),
      Type::Primitive(PrimitiveType::Numeric(numeric)) => {
        let bytes = shape_size(shape)?;
        let kind = match numeric {
          NumericType::Integer { signed } if matches!(bytes, 1 | 2 | 4 | 8) => {
            CPrimitive::Int { bytes, signed }
          }
          NumericType::Float if matches!(bytes, 4 | 8) => CPrimitive::Float { bytes },
          _ => {
            return Err(FfiError(format!(
              "{} has no C equivalent",
              shape.type_identifier
            )));
          }
        };
        Ok(CType::Primitive { kind, shape })
      }
      Type::User(UserType::Struct(s)) => {
        let c_name = self.alloc_name(shape);
        let s = self.build_struct(c_name, s.fields)?;
        Ok(CType::Struct(Arc::new(s)))
      }
      Type::User(UserType::Enum(e)) => self.build_enum(shape, e),
      _ => Err(FfiError(format!(
        "{} is not supported (opaque, add a facet proxy for it)",
        shape.type_identifier
      ))),
    }
  }

  fn build_proxy(
    &mut self,
    def: &'static facet::ProxyDef,
    field_level: bool,
  ) -> Result<CType, FfiError> {
    let proxy = Box::new(self.build(def.shape)?);
    Ok(CType::Proxy {
      def,
      field_level,
      proxy,
    })
  }

  fn build_struct(
    &mut self,
    c_name: String,
    fields: &'static [facet::Field],
  ) -> Result<CStruct, FfiError> {
    let mut c_fields = Vec::new();
    let mut zst_fields = Vec::new();
    let mut offset = 0;
    let mut align = 1;
    for (rust_index, field) in fields.iter().enumerate() {
      let field_shape = field.shape();
      let ty = match field.proxy() {
        Some(def) => self.build_proxy(def, true),
        None if shape_size(field_shape)? == 0 => {
          zst_fields.push(rust_index);
          continue;
        }
        None => self.build(field_shape),
      };
      let ty = ty.map_err(|e| FfiError(format!("{c_name}.{}: {e}", field.name)))?;
      offset = align_up(offset, ty.align());
      align = align.max(ty.align());
      c_fields.push(CField {
        c_name: sanitize_ident(field.name),
        rust_index,
        rust_offset: field.offset,
        offset,
        ty: ty.clone(),
      });
      offset += ty.size();
    }
    if c_fields.is_empty() {
      return Err(FfiError(format!("{c_name} has no C visible field")));
    }
    Ok(CStruct {
      c_name,
      fields: c_fields,
      zst_fields,
      size: align_up(offset, align),
      align,
    })
  }

  fn build_enum(&mut self, shape: &'static Shape, e: facet::EnumType) -> Result<CType, FfiError> {
    if matches!(e.enum_repr, EnumRepr::Rust | EnumRepr::RustNPO) {
      return Err(FfiError(format!(
        "enum {} needs an explicit repr",
        shape.type_identifier
      )));
    }
    let c_name = self.alloc_name(shape);

    // use the rust discriminant as the tag if it's representable, so the C side value matches the
    // rust side for field-less enums.
    let use_discriminant = e.variants.iter().all(|v| {
      v.discriminant
        .is_some_and(|d| (0..=u32::MAX as i64).contains(&d))
    });

    let mut variants = Vec::new();
    let mut payload_size = 0;
    let mut payload_align = 1;
    for (index, variant) in e.variants.iter().enumerate() {
      let tag = if use_discriminant {
        variant.discriminant.unwrap() as u32
      } else {
        index as u32
      };
      let payload = if variant.data.fields.is_empty() {
        None
      } else {
        let payload_name = format!("{c_name}_{}", sanitize_ident(variant.name));
        let payload = self.build_struct(payload_name, variant.data.fields)?;
        payload_size = payload_size.max(payload.size);
        payload_align = payload_align.max(payload.align);
        Some(Arc::new(payload))
      };
      variants.push(CVariant {
        c_name: sanitize_ident(variant.name),
        tag,
        payload,
      });
    }

    let tag_size = size_of::<u32>();
    let has_payload = variants.iter().any(|v| v.payload.is_some());
    let (payload_offset, size, align) = if has_payload {
      let align = payload_align.max(tag_size);
      let payload_offset = align_up(tag_size, payload_align);
      (
        payload_offset,
        align_up(payload_offset + payload_size, align),
        align,
      )
    } else {
      (tag_size, tag_size, tag_size)
    };

    Ok(CType::Enum(Arc::new(CEnum {
      c_name,
      variants,
      payload_offset,
      payload_size,
      size,
      align,
    })))
  }

  fn alloc_name(&mut self, shape: &'static Shape) -> String {
    let base = mangled_type_name(shape);
    let mut name = base.clone();
    let mut counter = 1;
    while let Some(id) = self.used_names.get(&name) {
      if *id == shape.id {
        return name;
      }
      counter += 1;
      name = format!("{base}_{counter}");
    }
    self.used_names.insert(name.clone(), shape.id);
    name
  }
}

/// bool is excluded because not every bit pattern is a valid bool.
fn bulk_copy_compatible(ty: &CType, shape: &'static Shape) -> bool {
  let Ok(rust_size) = shape_size(shape) else {
    return false;
  };
  match ty {
    CType::Primitive { kind, .. } => *kind != CPrimitive::Bool,
    CType::Array { element, .. } => match shape.def {
      Def::Array(a) => rust_size == ty.size() && bulk_copy_compatible(element, a.t()),
      _ => false,
    },
    CType::Struct(s) => {
      let Type::User(UserType::Struct(rust)) = shape.ty else {
        return false;
      };
      rust_size == s.size
        && s.fields.iter().all(|f| {
          f.offset == f.rust_offset
            && bulk_copy_compatible(&f.ty, rust.fields[f.rust_index].shape())
        })
    }
    _ => false,
  }
}

fn shape_size(shape: &'static Shape) -> Result<usize, FfiError> {
  shape
    .layout
    .sized_layout()
    .map(|l| l.size())
    .map_err(|_| FfiError(format!("{} is unsized", shape.type_identifier)))
}

/// `Vec3<f32>` => `Vec3_f32`
fn mangled_type_name(shape: &'static Shape) -> String {
  let mut name = sanitize_ident(shape.type_identifier);
  if let Type::User(UserType::Struct(s)) = shape.ty
    && matches!(s.kind, StructKind::Tuple)
  {
    name = "Tuple".to_string();
  }
  for param in shape.type_params {
    name.push('_');
    name.push_str(&mangled_type_name(param.shape));
  }
  name
}

pub fn sanitize_ident(name: &str) -> String {
  let mut out = String::with_capacity(name.len());
  for c in name.chars() {
    if c.is_ascii_alphanumeric() || c == '_' {
      out.push(c);
    } else if !out.ends_with('_') {
      out.push('_');
    }
  }
  let mut out = out.trim_matches('_').to_string();
  if out.is_empty() {
    out.push_str("unnamed");
  }
  if out.starts_with(|c: char| c.is_ascii_digit()) {
    out.insert(0, '_');
  }
  if CPP_KEYWORDS.contains(&out.as_str()) {
    out.push('_');
  }
  out
}

const CPP_KEYWORDS: &[&str] = &[
  "alignas",
  "alignof",
  "and",
  "asm",
  "auto",
  "bool",
  "break",
  "case",
  "catch",
  "char",
  "class",
  "const",
  "constexpr",
  "continue",
  "default",
  "delete",
  "do",
  "double",
  "else",
  "enum",
  "explicit",
  "export",
  "extern",
  "false",
  "float",
  "for",
  "friend",
  "goto",
  "if",
  "inline",
  "int",
  "long",
  "mutable",
  "namespace",
  "new",
  "not",
  "operator",
  "or",
  "private",
  "protected",
  "public",
  "register",
  "return",
  "short",
  "signed",
  "sizeof",
  "static",
  "struct",
  "switch",
  "template",
  "this",
  "throw",
  "true",
  "try",
  "typedef",
  "typename",
  "union",
  "unsigned",
  "using",
  "virtual",
  "void",
  "volatile",
  "while",
  "xor",
];
