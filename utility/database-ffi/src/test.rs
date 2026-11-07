use std::num::NonZeroU32;

use serde::*;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct Deg {
  pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct Projection {
  pub near: f32,
  pub far: f32,
  pub fov: Deg,
  pub flipped: bool,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub enum Mode {
  #[default]
  A = 0,
  B = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub enum Primitive {
  #[default]
  Empty,
  Circle {
    radius: f32,
  },
  Pair(u32, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct Nested {
  pub size: Option<NonZeroU32>,
  pub mat: [[f32; 2]; 2],
  pub mode: Mode,
  pub primitive: Primitive,
  pub ids: [u16; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct Vertex {
  pub position: [f32; 3],
  pub color: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct Mesh {
  pub name: String,
  pub label: Option<String>,
  pub vertices: Vec<Vertex>,
  pub flags: Vec<bool>,
  pub shared: std::sync::Arc<Vec<u16>>,
  pub tags: Vec<String>,
}

/// a foreign like type without facet, the valid range is 0..=10
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Level(u8);

impl Default for Level {
  fn default() -> Self {
    Level(3)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet)]
#[facet(transparent)]
pub struct LevelProxy(pub u32);

impl TryFrom<LevelProxy> for Level {
  type Error = String;
  fn try_from(value: LevelProxy) -> Result<Self, Self::Error> {
    if value.0 > 10 {
      return Err(format!("level {} is out of range", value.0));
    }
    Ok(Level(value.0 as u8))
  }
}

impl From<&Level> for LevelProxy {
  fn from(value: &Level) -> Self {
    LevelProxy(value.0 as u32)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
pub struct WithFieldProxy {
  #[facet(opaque, proxy = LevelProxy)]
  pub level: Level,
  pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet, Serialize, Deserialize)]
#[facet(proxy = ColorProxy)]
pub struct PackedColor(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Default, Facet)]
pub struct ColorProxy {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

impl From<ColorProxy> for PackedColor {
  fn from(c: ColorProxy) -> Self {
    PackedColor(u32::from_le_bytes([c.r, c.g, c.b, c.a]))
  }
}

impl From<&PackedColor> for ColorProxy {
  fn from(c: &PackedColor) -> Self {
    let [r, g, b, a] = c.0.to_le_bytes();
    ColorProxy { r, g, b, a }
  }
}

declare_entity!(FfiTestEntity);
declare_component!(FfiTestProjection, FfiTestEntity, Projection);
declare_component!(FfiTestNested, FfiTestEntity, Nested);
declare_component!(FfiTestName, FfiTestEntity, String);
declare_component!(FfiTestMesh, FfiTestEntity, Mesh);
declare_component!(FfiTestFieldProxy, FfiTestEntity, WithFieldProxy);
declare_component!(FfiTestColor, FfiTestEntity, PackedColor);
declare_component!(FfiTestColors, FfiTestEntity, Vec<PackedColor>);
declare_foreign_key!(FfiTestParent, FfiTestEntity, FfiTestEntity);

fn setup() -> (Database, FfiSchema) {
  let db = Database::new(true);
  db.declare_entity::<FfiTestEntity>()
    .declare_component::<FfiTestProjection>()
    .declare_component::<FfiTestNested>()
    .declare_component::<FfiTestName>()
    .declare_component::<FfiTestMesh>()
    .declare_component::<FfiTestFieldProxy>()
    .declare_component::<FfiTestColor>()
    .declare_component::<FfiTestColors>()
    .declare_foreign_key::<FfiTestParent>();
  let schema = FfiSchema::from_database(&db);
  (db, schema)
}

fn key<C: ComponentSemantic>() -> u64 {
  stable_hash(C::unique_name())
}

fn entity_key() -> u64 {
  stable_hash(FfiTestEntity::unique_name())
}

/// write the value by the typed api, read it into C, write the C value back to a new entity by
/// ffi, and read it again by the typed api.
fn roundtrip<C: ComponentSemantic>(db: &Database, schema: &FfiSchema, value: C::Data) -> Vec<u8> {
  let mut writer = db.entity_writer::<C::Entity>();
  let src = writer.new_entity(|w| w.write::<C>(&value)).into_raw();
  let dst = writer.new_entity(|w| w).into_raw();
  drop(writer);

  let c = schema.component(key::<C>()).unwrap();
  let mut buffer = zeroed_c_buffer(&c.value);
  let ptr = buffer.as_mut_ptr() as *mut u8;
  let mut arena = FfiArena::default();
  unsafe {
    let status = schema.read_component(db, c.key, src, ptr, &mut arena);
    assert_eq!(status, FfiStatus::Ok);
    assert_eq!(schema.write_component(db, c.key, dst, ptr), FfiStatus::Ok);
  }

  let dst = unsafe { EntityHandle::<C::Entity>::from_raw(dst) };
  assert_eq!(db.read::<C>().get(dst), Some(&value));
  buffer
    .iter()
    .flat_map(|v| v.to_ne_bytes())
    .take(c.value.size())
    .collect()
}

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
  f32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn component_struct<C: ComponentSemantic>(schema: &FfiSchema) -> Arc<CStruct> {
  match schema.component(key::<C>()).unwrap().value.c_visible() {
    CType::Struct(s) => s.clone(),
    _ => panic!("expect struct"),
  }
}

#[test]
fn schema_should_reflect_all_components() {
  let (_, schema) = setup();
  assert!(schema.unsupported.is_empty());
  assert!(
    schema
      .component(key::<FfiTestName>())
      .unwrap()
      .has_heap_data
  );
  assert!(
    !schema
      .component(key::<FfiTestNested>())
      .unwrap()
      .has_heap_data
  );
}

#[test]
fn repr_rust_struct_should_use_c_layout() {
  let (db, schema) = setup();
  let value = Projection {
    near: 0.1,
    far: 100.,
    fov: Deg { value: 45. },
    flipped: true,
  };
  let bytes = roundtrip::<FfiTestProjection>(&db, &schema, value);
  assert_eq!(bytes.len(), 16);
  assert_eq!(f32_at(&bytes, 0), 0.1);
  assert_eq!(f32_at(&bytes, 4), 100.);
  assert_eq!(f32_at(&bytes, 8), 45.);
  assert_eq!(bytes[12], 1);
}

#[test]
fn nested_value_should_roundtrip() {
  let (db, schema) = setup();
  for value in [
    Nested::default(),
    Nested {
      size: NonZeroU32::new(7),
      mat: [[1., 2.], [3., 4.]],
      mode: Mode::B,
      primitive: Primitive::Circle { radius: 2. },
      ids: [1, 2, 3],
    },
    Nested {
      primitive: Primitive::Pair(3, 0.5),
      ..Default::default()
    },
  ] {
    roundtrip::<FfiTestNested>(&db, &schema, value);
  }
}

#[test]
fn foreign_key_should_roundtrip() {
  let (db, schema) = setup();
  let target = db.entity_writer::<FfiTestEntity>().new_entity(|w| w);
  roundtrip::<FfiTestParent>(&db, &schema, Some(target.into_raw()));
  roundtrip::<FfiTestParent>(&db, &schema, None);
}

#[test]
fn heap_data_should_roundtrip() {
  let (db, schema) = setup();
  roundtrip::<FfiTestName>(&db, &schema, "名字 name".to_string());
  roundtrip::<FfiTestName>(&db, &schema, String::new());

  let mesh = Mesh {
    name: "mesh".into(),
    label: Some("label".into()),
    vertices: (0..100)
      .map(|i| Vertex {
        position: [i as f32, 1., 2.],
        color: i,
      })
      .collect(),
    flags: vec![true, false, true],
    shared: std::sync::Arc::new(vec![1, 2, 3]),
    tags: vec!["a".into(), "".into(), "c".into()],
  };
  roundtrip::<FfiTestMesh>(&db, &schema, mesh);
  roundtrip::<FfiTestMesh>(&db, &schema, Mesh::default());

  let mesh = component_struct::<FfiTestMesh>(&schema);
  let bulk_copy = |name: &str| match &mesh.fields.iter().find(|f| f.c_name == name).unwrap().ty {
    CType::Slice { bulk_copy, .. } => *bulk_copy,
    _ => panic!("expect slice"),
  };
  assert!(bulk_copy("vertices"));
  assert!(!bulk_copy("flags"));
  assert!(convert::list_direct_fill(<Vec<Vertex>>::SHAPE).is_some());
}

#[test]
fn read_string_should_be_nul_terminated() {
  let (db, schema) = setup();
  let entity = db
    .entity_writer::<FfiTestEntity>()
    .new_entity(|w| w.write::<FfiTestName>(&"abc".to_string()));
  let mut arena = FfiArena::default();
  let mut value = CSlice {
    ptr: std::ptr::null(),
    len: 0,
  };
  let ptr = &mut value as *mut CSlice as *mut u8;
  let status = unsafe {
    schema.read_component(
      &db,
      key::<FfiTestName>(),
      entity.into_raw(),
      ptr,
      &mut arena,
    )
  };
  assert_eq!(status, FfiStatus::Ok);
  let c_str = unsafe { std::ffi::CStr::from_ptr(value.ptr as *const std::ffi::c_char) };
  assert_eq!(c_str.to_str().unwrap(), "abc");
  assert_eq!(value.len, 3);
}

#[test]
fn proxy_should_roundtrip() {
  let (db, schema) = setup();
  let value = WithFieldProxy {
    level: Level(7),
    weight: 0.5,
  };
  let bytes = roundtrip::<FfiTestFieldProxy>(&db, &schema, value);
  assert_eq!(bytes[0], 7);

  let bytes = roundtrip::<FfiTestColor>(&db, &schema, PackedColor(0x04030201));
  assert_eq!(bytes, vec![1, 2, 3, 4]);

  let colors = vec![PackedColor(1), PackedColor(0xff000000)];
  roundtrip::<FfiTestColors>(&db, &schema, colors);
}

#[test]
fn invalid_c_value_should_be_rejected() {
  let (db, schema) = setup();
  let entity = schema.create_entity(&db, entity_key()).unwrap();
  let write = |c: u64, ptr: *const u8| unsafe { schema.write_component(&db, c, entity, ptr) };

  let mut bytes = [0u32; 4];
  bytes[3] = 2;
  let status = write(key::<FfiTestProjection>(), bytes.as_ptr() as *const u8);
  assert_eq!(status, FfiStatus::InvalidValue);

  let c = schema.component(key::<FfiTestNested>()).unwrap();
  let mut buffer = zeroed_c_buffer(&c.value);
  let ptr = buffer.as_mut_ptr() as *mut u8;
  let mut arena = FfiArena::default();
  unsafe {
    let status = schema.read_component(&db, c.key, entity, ptr, &mut arena);
    assert_eq!(status, FfiStatus::Ok);
    // size: has_value = true, value = 0, which violates NonZero
    *ptr = 1;
  }
  assert_eq!(write(c.key, ptr), FfiStatus::InvalidValue);

  let invalid_utf8 = [0xffu8, 0xfe];
  let value = CSlice {
    ptr: invalid_utf8.as_ptr(),
    len: 2,
  };
  let status = write(key::<FfiTestName>(), &value as *const CSlice as *const u8);
  assert_eq!(status, FfiStatus::InvalidValue);

  let value = CSlice {
    ptr: std::ptr::null(),
    len: 2,
  };
  let status = write(key::<FfiTestName>(), &value as *const CSlice as *const u8);
  assert_eq!(status, FfiStatus::InvalidValue);

  let value = [11u32, 0];
  let status = write(key::<FfiTestFieldProxy>(), value.as_ptr() as *const u8);
  assert_eq!(status, FfiStatus::InvalidValue);
}

#[test]
fn dead_handle_and_unknown_key_should_be_reported() {
  let (db, schema) = setup();
  let entity = schema.create_entity(&db, entity_key()).unwrap();
  assert_eq!(
    schema.delete_entity(&db, entity_key(), entity),
    FfiStatus::Ok
  );
  assert_eq!(
    schema.delete_entity(&db, entity_key(), entity),
    FfiStatus::DeadHandle
  );

  let c = schema.component(key::<FfiTestProjection>()).unwrap();
  let mut buffer = [0u32; 4];
  let ptr = buffer.as_mut_ptr() as *mut u8;
  let mut arena = FfiArena::default();
  unsafe {
    let status = schema.read_component(&db, c.key, entity, ptr, &mut arena);
    assert_eq!(status, FfiStatus::DeadHandle);
    let status = schema.write_component(&db, c.key, entity, ptr);
    assert_eq!(status, FfiStatus::DeadHandle);
    let status = schema.read_component(&db, 0, entity, ptr, &mut arena);
    assert_eq!(status, FfiStatus::UnknownKey);
  }
}

#[test]
fn every_component_should_roundtrip_sample_values() {
  let (db, schema) = setup();
  let failures = verify_sample_roundtrip(&db, &schema);
  assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn header_should_contain_typed_declarations() {
  let (_, schema) = setup();
  let header = schema.generate_cpp_header(&CppHeaderOptions {
    namespace: "rdb".into(),
    includes: vec!["bindings.h".into()],
    entity_handle_type: "ViewerEntityHandle".into(),
    status_type: "RdbStatus".into(),
    snapshot_type: "RdbSnapshot".into(),
  });
  for expected in [
    "struct Projection {\n  float near;\n  float far;\n  types::Deg fov;\n  bool flipped;\n};",
    "enum class Mode : uint32_t {\n  A = 0,\n  B = 5,\n};",
    "  float mat[2][2];",
    "  Optional<uint32_t> size;",
    "  using Value = Optional<Handle<entity::FfiTestEntity>>;",
    "  union {\n    Primitive_Circle Circle;\n    Primitive_Pair Pair;\n  } payload;",
    "  Str name;\n  Optional<Str> label;\n  Slice<types::Vertex> vertices;",
    "  Slice<uint16_t> shared;\n  Slice<Str> tags;",
    "  uint32_t level;",
    "  using Value = types::ColorProxy;",
    "  using Value = Slice<types::ColorProxy>;",
    "  static constexpr bool has_heap_data = true;",
  ] {
    assert!(
      header.contains(expected),
      "missing {expected:?} in\n{header}"
    );
  }
}
