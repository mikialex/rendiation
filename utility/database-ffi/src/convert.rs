use facet_reflect::ReflectErrorKind;

use crate::*;

/// Owns the memory pointed by the heap parts ([CType::Str], [CType::Slice]) of the C values
/// written by [write_rust_value_to_c]. The pointers are valid until the arena is dropped.
#[derive(Default)]
pub struct FfiArena {
  chunks: Vec<Box<[u64]>>,
}

impl FfiArena {
  /// return zero initialized memory aligned to 8
  pub fn alloc(&mut self, size: usize) -> *mut u8 {
    let mut chunk = vec![0u64; size.div_ceil(8).max(1)].into_boxed_slice();
    let ptr = chunk.as_mut_ptr() as *mut u8;
    self.chunks.push(chunk);
    ptr
  }

  pub fn is_empty(&self) -> bool {
    self.chunks.is_empty()
  }
}

/// Build a rust value of `shape` from the C value at `src`, and pass the pointer of the built
/// value to `f`. The value is dropped after `f` returns.
///
/// # Safety
///
/// `ty` must be built from `shape`, `src` must point to a readable and aligned C value of `ty`.
pub unsafe fn with_rust_value_from_c<R>(
  shape: &'static Shape,
  ty: &CType,
  src: *const u8,
  f: impl FnOnce(*const ()) -> R,
) -> Result<R, FfiError> {
  let partial = unsafe { Partial::alloc_shape(shape) }.map_err(ffi_err)?;
  let partial = unsafe { build_from_c(partial, ty, src) }?;
  let value = partial.build().map_err(ffi_err)?;
  let ptr = value.peek().data().as_byte_ptr() as *const ();
  Ok(f(ptr))
}

/// Write the rust value at `src` into `dst` as the C value of `ty`, the heap parts are allocated
/// in `arena`.
///
/// # Safety
///
/// `ty` must be built from `shape`, `src` must point to a valid value of `shape`, `dst` must
/// point to a writable and aligned C value of `ty`.
pub unsafe fn write_rust_value_to_c(
  shape: &'static Shape,
  ty: &CType,
  src: *const (),
  dst: *mut u8,
  arena: &mut FfiArena,
) -> Result<(), FfiError> {
  let peek = unsafe { Peek::unchecked_new(PtrConst::new(src as *const u8), shape) };
  unsafe { write_c(peek, ty, dst, arena) }
}

type Building<'f> = Partial<'f, true>;

unsafe fn build_from_c<'f>(
  p: Building<'f>,
  ty: &CType,
  src: *const u8,
) -> Result<Building<'f>, FfiError> {
  unsafe {
    match ty {
      CType::Primitive { kind, shape } => {
        if *kind == CPrimitive::Bool && *src > 1 {
          return Err(FfiError(format!("invalid bool value {}", *src)));
        }
        p.set_shape(PtrConst::new(src), shape).map_err(ffi_err)
      }
      CType::EntityHandle => {
        let handle = RawEntityHandle::from((src as *const FfiEntityHandle).read());
        p.set_shape(PtrConst::new(&handle as *const _), RawEntityHandle::SHAPE)
          .map_err(ffi_err)
      }
      CType::Array { element, count } => {
        let mut p = p.init_array().map_err(ffi_err)?;
        let stride = element.size();
        for i in 0..*count {
          p = p.begin_nth_field(i).map_err(ffi_err)?;
          p = build_from_c(p, element, src.add(i * stride))?;
          p = p.end().map_err(ffi_err)?;
        }
        Ok(p)
      }
      CType::Struct(s) => build_struct_from_c(p, s, src),
      CType::Optional(inner) => match *src {
        0 => p.set_default().map_err(ffi_err),
        1 => {
          let p = p.begin_some().map_err(ffi_err)?;
          let p = build_from_c(p, inner, src.add(optional_value_offset(inner)))?;
          p.end().map_err(ffi_err)
        }
        v => Err(FfiError(format!("invalid optional flag {v}"))),
      },
      CType::Enum(e) => {
        let tag = (src as *const u32).read();
        let (index, variant) = e
          .variants
          .iter()
          .enumerate()
          .find(|(_, v)| v.tag == tag)
          .ok_or_else(|| FfiError(format!("invalid tag {tag} for {}", e.c_name)))?;
        let p = p.select_nth_variant(index).map_err(ffi_err)?;
        match &variant.payload {
          Some(payload) => build_struct_from_c(p, payload, src.add(e.payload_offset)),
          None => Ok(p),
        }
      }
      CType::Str => {
        let bytes = read_c_slice(src, 1)?;
        let s = std::str::from_utf8(bytes).map_err(ffi_err)?;
        p.set(s.to_string()).map_err(ffi_err)
      }
      CType::Slice {
        shape,
        element,
        bulk_copy,
      } => {
        let stride = element.size();
        let bytes = read_c_slice(src, stride)?;
        let len = bytes.len() / stride.max(1);
        if *bulk_copy && let Some(fill) = list_direct_fill(shape) {
          return bulk_build_list(p, fill, bytes, len);
        }
        let mut p = p.init_list().map_err(ffi_err)?;
        for i in 0..len {
          p = p.begin_list_item().map_err(ffi_err)?;
          p = build_from_c(p, element, bytes.as_ptr().add(i * stride))?;
          p = p.end().map_err(ffi_err)?;
        }
        Ok(p)
      }
      CType::Inner { inner, .. } => {
        let p = p.begin_inner().map_err(ffi_err)?;
        let p = build_from_c(p, inner, src)?;
        p.end().map_err(ffi_err)
      }
      CType::Pointer { pointee } => {
        let p = p.begin_smart_ptr().map_err(ffi_err)?;
        let p = build_from_c(p, pointee, src)?;
        p.end().map_err(ffi_err)
      }
      CType::Proxy {
        field_level, proxy, ..
      } => {
        let p = if *field_level {
          p.begin_custom_deserialization().map_err(ffi_err)?
        } else {
          let (p, has_proxy) = p
            .begin_custom_deserialization_from_shape()
            .map_err(ffi_err)?;
          if !has_proxy {
            return Err(FfiError("the container proxy is not found".into()));
          }
          p
        };
        let p = build_from_c(p, proxy, src)?;
        p.end().map_err(ffi_err)
      }
    }
  }
}

pub(crate) struct ListDirectFill {
  init: facet::ListInitInPlaceWithCapacityFn,
  as_mut_ptr: facet::ListAsMutPtrTypedFn,
  set_len: facet::ListSetLenFn,
}

pub(crate) fn list_direct_fill(shape: &'static Shape) -> Option<ListDirectFill> {
  let Def::List(def) = shape.def else {
    return None;
  };
  Some(ListDirectFill {
    init: def.init_in_place_with_capacity()?,
    as_mut_ptr: def.as_mut_ptr_typed()?,
    set_len: def.set_len()?,
  })
}

/// Fill the whole list by one copy, only valid if the element is [CType::Slice::bulk_copy].
unsafe fn bulk_build_list<'f>(
  p: Building<'f>,
  fill: ListDirectFill,
  bytes: &[u8],
  len: usize,
) -> Result<Building<'f>, FfiError> {
  let fill = move |dst: facet::PtrUninit| -> Result<(), ReflectErrorKind> {
    unsafe {
      let list = (fill.init)(dst, len);
      std::ptr::copy_nonoverlapping(bytes.as_ptr(), (fill.as_mut_ptr)(list), bytes.len());
      (fill.set_len)(list, len);
    }
    Ok(())
  };
  unsafe { p.set_from_function(fill) }.map_err(ffi_err)
}

unsafe fn read_c_slice<'a>(src: *const u8, stride: usize) -> Result<&'a [u8], FfiError> {
  let slice = unsafe { (src as *const CSlice).read() };
  if slice.len == 0 {
    return Ok(&[]);
  }
  if slice.ptr.is_null() {
    return Err(FfiError("null pointer with non zero length".into()));
  }
  let byte_len = slice
    .len
    .checked_mul(stride)
    .ok_or_else(|| FfiError("slice length overflow".into()))?;
  Ok(unsafe { std::slice::from_raw_parts(slice.ptr, byte_len) })
}

unsafe fn build_struct_from_c<'f>(
  mut p: Building<'f>,
  s: &CStruct,
  src: *const u8,
) -> Result<Building<'f>, FfiError> {
  for field in &s.fields {
    p = p.begin_nth_field(field.rust_index).map_err(ffi_err)?;
    p = unsafe { build_from_c(p, &field.ty, src.add(field.offset)) }?;
    p = p.end().map_err(ffi_err)?;
  }
  for &index in &s.zst_fields {
    p = p.set_nth_field_to_default(index).map_err(ffi_err)?;
  }
  Ok(p)
}

unsafe fn write_c(
  peek: Peek,
  ty: &CType,
  dst: *mut u8,
  arena: &mut FfiArena,
) -> Result<(), FfiError> {
  unsafe {
    match ty {
      CType::Primitive { kind, .. } => {
        std::ptr::copy_nonoverlapping(peek.data().as_byte_ptr(), dst, kind.size());
      }
      CType::EntityHandle => {
        let handle = *peek.data().get::<RawEntityHandle>();
        (dst as *mut FfiEntityHandle).write(handle.into());
      }
      CType::Array { element, count } => {
        let list = peek.into_list_like().map_err(ffi_err)?;
        let stride = element.size();
        for i in 0..*count {
          let item = list
            .get(i)
            .ok_or_else(|| FfiError(format!("array item {i} is missing")))?;
          write_c(item, element, dst.add(i * stride), arena)?;
        }
      }
      CType::Struct(s) => {
        let peek = peek.into_struct().map_err(ffi_err)?;
        for field in &s.fields {
          let value = peek.field(field.rust_index).map_err(ffi_err)?;
          write_c(value, &field.ty, dst.add(field.offset), arena)?;
        }
      }
      CType::Optional(inner) => {
        let value_ptr = dst.add(optional_value_offset(inner));
        match peek.into_option().map_err(ffi_err)?.value() {
          Some(value) => {
            *dst = 1;
            write_c(value, inner, value_ptr, arena)?;
          }
          None => {
            *dst = 0;
            std::ptr::write_bytes(value_ptr, 0, inner.size());
          }
        }
      }
      CType::Enum(e) => {
        let peek = peek.into_enum().map_err(ffi_err)?;
        let variant = &e.variants[peek.variant_index().map_err(ffi_err)?];
        (dst as *mut u32).write(variant.tag);
        if let Some(payload) = &variant.payload {
          let payload_ptr = dst.add(e.payload_offset);
          std::ptr::write_bytes(payload_ptr, 0, e.payload_size);
          for field in &payload.fields {
            let value = peek
              .field(field.rust_index)
              .map_err(ffi_err)?
              .ok_or_else(|| FfiError(format!("{} variant field is missing", e.c_name)))?;
            write_c(value, &field.ty, payload_ptr.add(field.offset), arena)?;
          }
        }
      }
      CType::Str => {
        let s = peek
          .as_str()
          .ok_or_else(|| FfiError("expect a string".into()))?;
        let ptr = arena.alloc(s.len() + 1);
        std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
        (dst as *mut CSlice).write(CSlice { ptr, len: s.len() });
      }
      CType::Slice {
        element, bulk_copy, ..
      } => {
        let list = peek.into_list().map_err(ffi_err)?;
        let len = list.len();
        let stride = element.size();
        let ptr = if len == 0 {
          std::ptr::null_mut()
        } else {
          let ptr = arena.alloc(len * stride);
          if *bulk_copy {
            let first = list.get(0).unwrap().data().as_byte_ptr();
            std::ptr::copy_nonoverlapping(first, ptr, len * stride);
          } else {
            for (i, item) in list.iter().enumerate() {
              write_c(item, element, ptr.add(i * stride), arena)?;
            }
          }
          ptr
        };
        (dst as *mut CSlice).write(CSlice { ptr, len });
      }
      CType::Inner { shape, inner } => {
        let inner_shape = shape
          .inner
          .ok_or_else(|| FfiError(format!("{} has no inner", shape.type_identifier)))?;
        let inner_ptr = shape
          .call_try_borrow_inner(peek.data())
          .ok_or_else(|| FfiError(format!("{} can not borrow inner", shape.type_identifier)))?
          .map_err(FfiError)?;
        let inner_peek = Peek::unchecked_new(inner_ptr.as_const(), inner_shape);
        write_c(inner_peek, inner, dst, arena)?;
      }
      CType::Pointer { pointee } => {
        let inner = peek
          .into_pointer()
          .map_err(ffi_err)?
          .borrow_inner()
          .ok_or_else(|| FfiError("can not borrow the pointee".into()))?;
        write_c(inner, pointee, dst, arena)?;
      }
      CType::Proxy { def, proxy, .. } => {
        let proxy_shape = def.shape;
        let uninit = proxy_shape.allocate().map_err(ffi_err)?;
        let value = match (def.convert_out)(peek.data(), uninit) {
          Ok(value) => value,
          Err(e) => {
            proxy_shape.deallocate_uninit(uninit).ok();
            return Err(FfiError(e));
          }
        };
        let r = write_c(
          Peek::unchecked_new(value.as_const(), proxy_shape),
          proxy,
          dst,
          arena,
        );
        proxy_shape.call_drop_in_place(value);
        proxy_shape.deallocate_mut(value).ok();
        r?;
      }
    }
  }
  Ok(())
}
