use crate::*;

/// Write sample C values into every component through ffi and read them back, return the
/// description of all failures.
pub fn verify_sample_roundtrip(db: &Database, schema: &FfiSchema) -> Vec<String> {
  let mut failures = Vec::new();
  for c in &schema.components {
    let entity = schema.entity_by_id(c.entity).unwrap().key;
    let entity = schema.create_entity(db, entity).unwrap();
    for seed in 0..4 {
      let mut sample_arena = FfiArena::default();
      let mut read_arena = FfiArena::default();
      let mut sample = zeroed_c_buffer(&c.value);
      let mut read = zeroed_c_buffer(&c.value);
      let sample_ptr = sample.as_mut_ptr() as *mut u8;
      let read_ptr = read.as_mut_ptr() as *mut u8;
      let result = unsafe {
        fill_sample_c_value(&c.value, sample_ptr, seed, &mut sample_arena).and_then(|_| {
          let write = schema.write_component(db, c.key, entity, sample_ptr);
          let read = schema.read_component(db, c.key, entity, read_ptr, &mut read_arena);
          if write != FfiStatus::Ok || read != FfiStatus::Ok {
            return Err(FfiError(format!("write {write:?}, read {read:?}")));
          }
          if !c_value_eq(&c.value, sample_ptr, read_ptr) {
            return Err(FfiError("the read value is different".into()));
          }
          Ok(())
        })
      };
      if let Err(e) = result {
        failures.push(format!("{} seed {seed}: {e}", c.name));
        break;
      }
    }
  }
  failures
}

/// Fill a deterministic and valid C value. The field level proxy (usually for an opaque foreign
/// type) is filled by the proxy's default value, because the valid proxy value is defined by the
/// conversion.
///
/// # Safety
///
/// `dst` must point to a zero initialized, writable and aligned C value of `ty`.
pub unsafe fn fill_sample_c_value(
  ty: &CType,
  dst: *mut u8,
  seed: u32,
  arena: &mut FfiArena,
) -> Result<(), FfiError> {
  unsafe {
    match ty {
      CType::Primitive { kind, .. } => match kind {
        CPrimitive::Bool => *dst = (seed % 2) as u8,
        CPrimitive::Int { bytes, .. } => {
          let v = (seed as u64 % 100 + 1).to_ne_bytes();
          let start = if cfg!(target_endian = "big") {
            8 - bytes
          } else {
            0
          };
          std::ptr::copy_nonoverlapping(v.as_ptr().add(start), dst, *bytes);
        }
        CPrimitive::Float { bytes: 4 } => (dst as *mut f32).write(seed as f32 * 0.5 + 0.25),
        CPrimitive::Float { .. } => (dst as *mut f64).write(seed as f64 * 0.5 + 0.25),
      },
      CType::EntityHandle => (dst as *mut FfiEntityHandle).write(FfiEntityHandle {
        index: seed + 1,
        generation: seed as u64 + 7,
      }),
      CType::Array { element, count } => {
        for i in 0..*count {
          fill_sample_c_value(element, dst.add(i * element.size()), seed + i as u32, arena)?;
        }
      }
      CType::Struct(s) => fill_struct(s, dst, seed, arena)?,
      CType::Optional(inner) => {
        if seed % 2 == 1 {
          *dst = 1;
          let value = dst.add(optional_value_offset(inner));
          fill_sample_c_value(inner, value, seed, arena)?;
        }
      }
      CType::Enum(e) => {
        let variant = &e.variants[seed as usize % e.variants.len()];
        (dst as *mut u32).write(variant.tag);
        if let Some(payload) = &variant.payload {
          fill_struct(payload, dst.add(e.payload_offset), seed, arena)?;
        }
      }
      CType::Str => {
        let s = format!("sample {seed}");
        let ptr = arena.alloc(s.len() + 1);
        std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
        (dst as *mut CSlice).write(CSlice { ptr, len: s.len() });
      }
      CType::Slice { element, .. } => {
        let len = seed as usize % 3;
        let stride = element.size();
        let ptr = arena.alloc(len * stride);
        for i in 0..len {
          fill_sample_c_value(element, ptr.add(i * stride), seed + i as u32, arena)?;
        }
        (dst as *mut CSlice).write(CSlice { ptr, len });
      }
      CType::Inner { inner, .. } => fill_sample_c_value(inner, dst, seed, arena)?,
      CType::Pointer { pointee } => fill_sample_c_value(pointee, dst, seed, arena)?,
      CType::Proxy {
        def,
        field_level: true,
        proxy,
      } => {
        let default = Partial::alloc_shape(def.shape).map_err(ffi_err)?;
        let default = default
          .set_default()
          .and_then(|p| p.build())
          .map_err(ffi_err)?;
        let ptr = default.peek().data().as_byte_ptr() as *const ();
        write_rust_value_to_c(def.shape, proxy, ptr, dst, arena)?;
      }
      CType::Proxy { proxy, .. } => fill_sample_c_value(proxy, dst, seed, arena)?,
    }
  }
  Ok(())
}

unsafe fn fill_struct(
  s: &CStruct,
  dst: *mut u8,
  seed: u32,
  arena: &mut FfiArena,
) -> Result<(), FfiError> {
  for (i, field) in s.fields.iter().enumerate() {
    let ptr = unsafe { dst.add(field.offset) };
    unsafe { fill_sample_c_value(&field.ty, ptr, seed + i as u32, arena) }?;
  }
  Ok(())
}

/// Compare two C values structurally, the heap parts are compared by content.
///
/// # Safety
///
/// `a` and `b` must point to valid C values of `ty`.
pub unsafe fn c_value_eq(ty: &CType, a: *const u8, b: *const u8) -> bool {
  unsafe {
    match ty.c_visible() {
      CType::Primitive { kind, .. } => bytes_eq(a, b, kind.size()),
      CType::EntityHandle => {
        (a as *const FfiEntityHandle).read() == (b as *const FfiEntityHandle).read()
      }
      CType::Array { element, count } => (0..*count).all(|i| {
        let offset = i * element.size();
        c_value_eq(element, a.add(offset), b.add(offset))
      }),
      CType::Struct(s) => struct_eq(s, a, b),
      CType::Optional(inner) => {
        let offset = optional_value_offset(inner);
        *a == *b && (*a == 0 || c_value_eq(inner, a.add(offset), b.add(offset)))
      }
      CType::Enum(e) => {
        let tag = (a as *const u32).read();
        if tag != (b as *const u32).read() {
          return false;
        }
        let variant = e.variants.iter().find(|v| v.tag == tag);
        match variant.and_then(|v| v.payload.as_ref()) {
          Some(payload) => struct_eq(payload, a.add(e.payload_offset), b.add(e.payload_offset)),
          None => true,
        }
      }
      CType::Str => {
        let (a, b) = ((a as *const CSlice).read(), (b as *const CSlice).read());
        a.len == b.len && (a.len == 0 || bytes_eq(a.ptr, b.ptr, a.len))
      }
      CType::Slice { element, .. } => {
        let (a, b) = ((a as *const CSlice).read(), (b as *const CSlice).read());
        a.len == b.len
          && (0..a.len).all(|i| {
            let offset = i * element.size();
            c_value_eq(element, a.ptr.add(offset), b.ptr.add(offset))
          })
      }
      _ => unreachable!(),
    }
  }
}

unsafe fn struct_eq(s: &CStruct, a: *const u8, b: *const u8) -> bool {
  s.fields
    .iter()
    .all(|f| unsafe { c_value_eq(&f.ty, a.add(f.offset), b.add(f.offset)) })
}

unsafe fn bytes_eq(a: *const u8, b: *const u8, len: usize) -> bool {
  unsafe { std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len) }
}

/// A zero initialized buffer aligned for any C value.
pub fn zeroed_c_buffer(ty: &CType) -> Vec<u64> {
  vec![0; ty.size().div_ceil(8)]
}
