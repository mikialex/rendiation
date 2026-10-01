use rendiation_shader_api::*;

use crate::binding::*;
use crate::harness::*;

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default)]
pub struct Std430Matrices {
  pub a: f32,
  pub m3x2: Mat3x2<f32>,
  pub b: f32,
  pub m4x2: Mat4x2<f32>,
  pub c: f32,
  pub m2x4: Mat2x4<f32>,
  pub m3x4: Mat3x4<f32>,
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, Default)]
pub struct Std140Matrices {
  pub a: f32,
  pub m2x4: Mat2x4<f32>,
  pub b: f32,
  pub m3x4: Mat3x4<f32>,
}

/// the host shareable non square matrices, the host layout must match the WGSL layout
#[test]
fn non_square_matrix_host_layout() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;

    let s = zeroed_val::<Std430Matrices>().expand();
    let s = ENode::<Std430Matrices> { a: f, ..s }.construct().expand();
    keep(s.m3x2 * s.b);
    keep(s.m4x2 * s.c);
    keep(s.m2x4);
    keep(s.m3x4);

    let s = zeroed_val::<Std140Matrices>().expand();
    let s = ENode::<Std140Matrices> { a: f, ..s }.construct().expand();
    keep(s.m2x4 * s.b);
    keep(s.m3x4);
  });
}

/// The natural alignment is 4, the std140 struct size is rounded up to 16, so the naga struct
/// has trailing padding members.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140A {
  pub x: f32,
}

/// std140 aligns the nested struct to 16 but its natural offset is 4, so there are padding
/// members before it.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140B {
  pub a: u32,
  pub inner: Std140A,
}

/// Three levels of padded nested structs, padded before and after the nested struct.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140C {
  pub a: f32,
  pub b: Std140B,
  pub c: f32,
}

/// The scalar after vec3, the natural layout is the host layout, so no padding member exists.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140Plain {
  pub v: Vec3<f32>,
  pub s: f32,
}

/// The natural alignment is 8 and it has no padding member, but it's padded by 8 bytes when
/// nested after a scalar.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140Pair {
  pub a: u32,
  pub b: Vec2<f32>,
}

/// Mixes the padded and non padded nested structs, the arrays of padded struct, vector and the
/// matrices.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct Std140Mixed {
  pub head: f32,
  pub c: Std140C,
  pub after_c: f32,
  pub pair: Std140Pair,
  pub plain: Std140Plain,
  pub b_arr: Shader140Array<Std140B, 2>,
  pub v3: Vec3<f32>,
  pub after_v3: u32,
  pub m3: Shader16PaddedMat3,
  pub m2x4: Mat2x4<f32>,
  pub v4_arr: Shader140Array<Vec4<f32>, 2>,
  pub v3_arr: Shader140Array<Vec3<f32>, 2>,
  pub tail: u32,
  pub flag: Bool,
}

pub fn std140_a(x: f32) -> Std140A {
  Std140A {
    x,
    ..Zeroable::zeroed()
  }
}

pub fn std140_b(a: u32, x: f32) -> Std140B {
  Std140B {
    a,
    inner: std140_a(x),
    ..Zeroable::zeroed()
  }
}

/// Every leaf value is distinct, so any misplaced field is detected.
pub fn std140_mixed_data() -> Std140Mixed {
  Std140Mixed {
    head: 1.,
    c: Std140C {
      a: 2.,
      b: std140_b(3, 4.),
      c: 5.,
      ..Zeroable::zeroed()
    },
    after_c: 6.,
    pair: Std140Pair {
      a: 7,
      b: Vec2::new(8., 9.),
      ..Zeroable::zeroed()
    },
    plain: Std140Plain {
      v: Vec3::new(10., 11., 12.),
      s: 13.,
      ..Zeroable::zeroed()
    },
    b_arr: [std140_b(14, 15.), std140_b(16, 17.)].into(),
    v3: Vec3::new(18., 19., 20.),
    after_v3: 21,
    m3: Mat3::from([22., 23., 24., 25., 26., 27., 28., 29., 30.]).into(),
    m2x4: Mat2x4::from([31., 32., 33., 34., 35., 36., 37., 38.]),
    v4_arr: [Vec4::new(39., 40., 41., 42.), Vec4::new(43., 44., 45., 46.)].into(),
    v3_arr: [Vec3::new(47., 48., 49.), Vec3::new(50., 51., 52.)].into(),
    tail: 53,
    flag: true.into(),
    ..Zeroable::zeroed()
  }
}

/// The leaf values of the struct in the field order, the shader side leaves must be the same.
pub fn std140_mixed_cpu_leaves(v: &Std140Mixed) -> Vec<f32> {
  let mut r = vec![
    v.head,
    v.c.a,
    v.c.b.a as f32,
    v.c.b.inner.x,
    v.c.c,
    v.after_c,
    v.pair.a as f32,
    v.pair.b.x,
    v.pair.b.y,
  ];
  r.extend([v.plain.v.x, v.plain.v.y, v.plain.v.z, v.plain.s]);
  for b in &v.b_arr.inner {
    r.extend([b.inner.a as f32, b.inner.inner.x]);
  }
  r.extend([v.v3.x, v.v3.y, v.v3.z, v.after_v3 as f32]);
  let m3: [f32; 9] = Mat3::from(v.m3).into();
  r.extend(m3);
  let m2x4: [f32; 8] = v.m2x4.into();
  r.extend(m2x4);
  for x in v.v4_arr.iter() {
    r.extend([x.x, x.y, x.z, x.w]);
  }
  for x in v.v3_arr.iter() {
    r.extend([x.x, x.y, x.z]);
  }
  r.push(v.tail as f32);
  r.push(bool::from(v.flag) as u32 as f32);
  r
}

fn vec2_leaves(r: &mut Vec<Node<f32>>, v: Node<Vec2<f32>>) {
  r.extend([v.x(), v.y()]);
}

fn vec3_leaves(r: &mut Vec<Node<f32>>, v: Node<Vec3<f32>>) {
  r.extend([v.x(), v.y(), v.z()]);
}

fn vec4_leaves(r: &mut Vec<Node<f32>>, v: Node<Vec4<f32>>) {
  r.extend([v.x(), v.y(), v.z(), v.w()]);
}

fn bool_leaf(v: Node<Bool>) -> Node<f32> {
  v.into_bool().select(val(1.), val(0.))
}

/// How the fixed size array field is accessed through the pointer.
#[derive(Clone, Copy)]
pub enum ArrayAccess {
  /// index the array pointer
  Index,
  /// load or store the whole array, for the pointer that does not support the array index
  Whole,
}

fn array_ptr<T: ShaderSizedValueNodeType>(
  p: ShaderReadonlyPtrOf<T>,
  access: ArrayAccess,
) -> ShaderReadonlyPtrOf<T> {
  match access {
    ArrayAccess::Index => p,
    ArrayAccess::Whole => as_readonly(&p.load().make_local_var()),
  }
}

/// The leaves read through the pointer of each field, in the same order of the cpu leaves. The
/// array index is runtime when the `zero` is runtime.
pub fn std140_mixed_ptr_leaves(
  p: &ShaderReadonlyPtrOf<Std140Mixed>,
  zero: Node<u32>,
  access: ArrayAccess,
) -> Vec<Node<f32>> {
  let mut r = vec![
    p.head().load(),
    p.c().a().load(),
    p.c().b().a().load().into_f32(),
    p.c().b().inner().x().load(),
    p.c().c().load(),
    p.after_c().load(),
    p.pair().a().load().into_f32(),
  ];
  vec2_leaves(&mut r, p.pair().b().load());
  vec3_leaves(&mut r, p.plain().v().load());
  r.push(p.plain().s().load());
  let b_arr = array_ptr::<[Std140B; 2]>(p.b_arr(), access);
  for i in 0..2 {
    let b = b_arr.index(zero + val(i));
    r.push(b.a().load().into_f32());
    r.push(b.inner().x().load());
  }
  vec3_leaves(&mut r, p.v3().load());
  r.push(p.after_v3().load().into_f32());
  let m3 = p.m3();
  vec3_leaves(&mut r, m3.x().load());
  vec3_leaves(&mut r, m3.y().load());
  vec3_leaves(&mut r, m3.z().load());
  vec4_leaves(&mut r, p.m2x4().x().load());
  vec4_leaves(&mut r, p.m2x4().y().load());
  let v4_arr = array_ptr::<[Vec4<f32>; 2]>(p.v4_arr(), access);
  for i in 0..2 {
    vec4_leaves(&mut r, v4_arr.index(zero + val(i)).load());
  }
  let v3_arr = array_ptr::<[Vec3<f32>; 2]>(p.v3_arr(), access);
  for i in 0..2 {
    vec3_leaves(&mut r, v3_arr.index(zero + val(i)).load());
  }
  r.push(p.tail().load().into_f32());
  r.push(bool_leaf(p.flag().load()));
  r
}

/// The leaves read from the value by expanding each nested struct, in the same order of the cpu
/// leaves.
pub fn std140_mixed_value_leaves(v: Node<Std140Mixed>, zero: Node<u32>) -> Vec<Node<f32>> {
  let e = v.expand();
  let c = e.c.expand();
  let b = c.b.expand();
  let pair = e.pair.expand();
  let plain = e.plain.expand();
  let mut r = vec![
    e.head,
    c.a,
    b.a.into_f32(),
    Std140A::x(b.inner),
    c.c,
    e.after_c,
    pair.a.into_f32(),
  ];
  vec2_leaves(&mut r, pair.b);
  vec3_leaves(&mut r, plain.v);
  r.push(plain.s);
  // the array of struct value can only be indexed through a pointer
  let b_arr = e.b_arr.make_local_var();
  for i in 0..2 {
    let b = b_arr.index(zero + val(i)).load().expand();
    r.push(b.a.into_f32());
    r.push(b.inner.expand().x);
  }
  vec3_leaves(&mut r, e.v3);
  r.push(e.after_v3.into_f32());
  vec3_leaves(&mut r, e.m3.x());
  vec3_leaves(&mut r, e.m3.y());
  vec3_leaves(&mut r, e.m3.z());
  vec4_leaves(&mut r, e.m2x4.x());
  vec4_leaves(&mut r, e.m2x4.y());
  for i in 0..2 {
    vec4_leaves(&mut r, e.v4_arr.index(zero + val(i)));
  }
  for i in 0..2 {
    vec3_leaves(&mut r, e.v3_arr.index(val(i)));
  }
  r.push(e.tail.into_f32());
  r.push(bool_leaf(e.flag));
  r
}

/// Copy the struct field by field through the pointers, at different granularity: the leaf
/// scalar, the nested struct, the matrix column, the array element and the whole array.
pub fn std140_mixed_store_by_fields(
  target: &ShaderPtrOf<Std140Mixed>,
  source: &ShaderReadonlyPtrOf<Std140Mixed>,
  zero: Node<u32>,
  access: ArrayAccess,
) {
  target.head().store(source.head().load());
  target.c().a().store(source.c().a().load());
  target.c().b().a().store(source.c().b().a().load());
  target
    .c()
    .b()
    .inner()
    .x()
    .store(source.c().b().inner().x().load());
  target.c().c().store(source.c().c().load());
  target.after_c().store(source.after_c().load());
  target.pair().store(source.pair().load());
  target.plain().v().store(source.plain().v().load());
  target.plain().s().store(source.plain().s().load());
  match access {
    ArrayAccess::Index => {
      for i in 0..2 {
        let index = zero + val(i);
        let (t, s) = (target.b_arr().index(index), source.b_arr().index(index));
        t.a().store(s.a().load());
        t.inner().store(s.inner().load());
      }
    }
    ArrayAccess::Whole => target.b_arr().store(source.b_arr().load()),
  }
  target.v3().store(source.v3().load());
  target.after_v3().store(source.after_v3().load());
  target.m3().store(source.m3().load());
  target.m2x4().x().store(source.m2x4().x().load());
  target.m2x4().y().store(source.m2x4().y().load());
  match access {
    ArrayAccess::Index => {
      for i in 0..2 {
        let index = zero + val(i);
        let v = source.v4_arr().index(index).load();
        target.v4_arr().index(index).store(v);
      }
    }
    ArrayAccess::Whole => target.v4_arr().store(source.v4_arr().load()),
  }
  target.v3_arr().store(source.v3_arr().load());
  target.tail().store(source.tail().load());
  target.flag().store(source.flag().load());
}

/// Rebuild the struct from the expanded fields at every nesting level.
pub fn std140_mixed_rebuild(v: Node<Std140Mixed>, zero: Node<u32>) -> Node<Std140Mixed> {
  let e = v.expand();
  let c = e.c.expand();
  let b = c.b.expand();
  let inner = ENode::<Std140A> {
    x: Std140A::x(b.inner),
  }
  .construct();
  let b = ENode::<Std140B> { inner, ..b }.construct();
  let c = ENode::<Std140C> { b, ..c }.construct();

  let b_arr = e.b_arr.make_local_var();
  for i in 0..2 {
    let item = b_arr.index(zero + val(i));
    let b = item.load().expand();
    let inner = b.inner.expand().construct();
    item.store(ENode::<Std140B> { inner, ..b }.construct());
  }

  ENode::<Std140Mixed> {
    c,
    pair: e.pair.expand().construct(),
    plain: e.plain.expand().construct(),
    b_arr: b_arr.load(),
    ..e
  }
  .construct()
}

/// Rebuild the struct, the padded fields are replaced by the constants of the same value.
pub fn std140_mixed_rebuild_by_constants(
  v: Node<Std140Mixed>,
  data: &Std140Mixed,
) -> Node<Std140Mixed> {
  ENode::<Std140Mixed> {
    c: val(data.c),
    pair: val(data.pair),
    b_arr: val(data.b_arr.into_shader_ty()),
    m3: val(Mat3::from(data.m3)),
    v3_arr: val(data.v3_arr.into_shader_ty()),
    ..v.expand()
  }
  .construct()
}

/// All the ways to access the std140 struct, each path returns all the leaves.
const STD140_ACCESS_PATH_COUNT: usize = 12;

/// The raw buffers are the uniform, readonly storage and read_write storage of [Std140Mixed], the
/// read_write storage is written field by field from the readonly storage.
fn std140_access_paths(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let zero = builder.global_invocation_id().x();
  let data = std140_mixed_data();
  let uniform = typed_readonly_ptr::<Std140Mixed>(&buffers[0]);
  let storage = typed_readonly_ptr::<Std140Mixed>(&buffers[1]);
  let rw_storage = typed_ptr::<Std140Mixed>(&buffers[2]);

  let mut r = std140_mixed_ptr_leaves(&uniform, zero, ArrayAccess::Index);
  r.extend(std140_mixed_ptr_leaves(&storage, zero, ArrayAccess::Index));

  let loaded = uniform.load();
  r.extend(std140_mixed_value_leaves(loaded, zero));

  let local = loaded.make_local_var();
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&local),
    zero,
    ArrayAccess::Index,
  ));

  let by_fields = make_local_var::<Std140Mixed>();
  std140_mixed_store_by_fields(&by_fields, &uniform, zero, ArrayAccess::Index);
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&by_fields),
    zero,
    ArrayAccess::Index,
  ));

  std140_mixed_store_by_fields(&rw_storage, &storage, zero, ArrayAccess::Index);
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&rw_storage),
    zero,
    ArrayAccess::Index,
  ));

  let private = private_var::<Std140Mixed>();
  private.store(loaded);
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&private),
    zero,
    ArrayAccess::Index,
  ));

  let shared = builder.define_workgroup_shared_var::<Std140Mixed>();
  shared.store(loaded);
  workgroup_barrier();
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&shared),
    zero,
    ArrayAccess::Index,
  ));
  r.extend(std140_mixed_value_leaves(
    workgroup_uniform_load(shared),
    zero,
  ));

  r.extend(std140_mixed_value_leaves(
    std140_mixed_rebuild(loaded, zero),
    zero,
  ));
  r.extend(std140_mixed_value_leaves(
    std140_mixed_rebuild_by_constants(loaded, &data),
    zero,
  ));
  r.extend(std140_mixed_value_leaves(val(data), zero));
  r
}

fn std140_access_path_buffers(data: &Std140Mixed) -> [RawBuffer; 3] {
  let ty = Std140Mixed::ty();
  let empty: Std140Mixed = Zeroable::zeroed();
  [
    RawBuffer {
      ty: ty.clone(),
      space: BufferSpace::Uniform,
      bytes: bytes_of(data).to_vec(),
    },
    RawBuffer {
      ty: ty.clone(),
      space: BufferSpace::Storage,
      bytes: bytes_of(data).to_vec(),
    },
    RawBuffer {
      ty,
      space: BufferSpace::ReadWriteStorage,
      bytes: bytes_of(&empty).to_vec(),
    },
  ]
}

/// the std140 struct with explicit padding members, accessed through the buffer bindings of each
/// address space, the local, private and workgroup variables, the loaded and rebuilt value and the
/// inlined constant
#[test]
fn std140_padded_struct_access_paths() {
  let data = std140_mixed_data();
  let buffers = std140_access_path_buffers(&data);
  check_raw_buffers(&buffers, std140_access_paths);
}

/// the GPU version of [std140_padded_struct_access_paths], every access path reads the host data,
/// and the field by field writes to the read_write storage land at the host offsets
#[pollster::test]
async fn std140_padded_struct_gpu_access_paths() {
  let data = std140_mixed_data();
  let buffers = std140_access_path_buffers(&data);
  let expect = std140_mixed_cpu_leaves(&data);
  assert_eq!(expect.len(), 54);

  let result = gpu_run_raw_buffers(
    &buffers,
    1,
    expect.len() * STD140_ACCESS_PATH_COUNT,
    std140_access_paths,
  )
  .await;
  for (path, leaves) in result.output[0].chunks(expect.len()).enumerate() {
    assert_eq!(leaves, expect, "access path {path}");
  }

  let written: Std140Mixed = pod_read_unaligned(&result.buffers[2]);
  assert_eq!(std140_mixed_cpu_leaves(&written), expect);
}

fn std140_named_constant_logic(
  builder: &ShaderComputePipelineBuilder,
  _: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let zero = builder.global_invocation_id().x();
  std140_mixed_value_leaves(global_const_val(std140_mixed_data()), zero)
}

/// the named constant (not inlined into the function) of the padded struct
#[test]
fn std140_padded_struct_named_constant() {
  check_raw_buffers(&[], std140_named_constant_logic);
}

/// the GPU version of [std140_padded_struct_named_constant]
#[pollster::test]
#[ignore = "bug: naga msl output of the named struct constant with a vec3 after a gap fails to compile on Metal"]
async fn std140_padded_struct_gpu_named_constant() {
  let expect = std140_mixed_cpu_leaves(&std140_mixed_data());
  let result = gpu_run_raw_buffers(&[], 1, expect.len(), std140_named_constant_logic).await;
  assert_eq!(result.output[0], expect);
}

fn compose_by_host_value_leaves() -> Node<Vec4<f32>> {
  let c = std140_mixed_data().c.to_shader_node_by_value().expand();
  let b = c.b.expand();
  let array = std140_mixed_data().b_arr.into_shader_ty();
  let array = array.to_shader_node_by_value().make_local_var();
  let inner = std430_inner(1., Vec3::new(2., 3., 4.)).to_shader_node_by_value();
  let x = Std140A::x(b.inner) + array.index(val(1)).inner().x().load();
  (c.a, x, c.c, Std430Inner::v(inner).z()).into()
}

/// the padded struct, the array of padded struct and the std430 struct composed from the host
/// value (not a constant) by `to_shader_node_by_value`
#[test]
fn compose_by_host_value() {
  check_compute(|_| keep(compose_by_host_value_leaves()));
}

/// the GPU version of [compose_by_host_value]
#[pollster::test]
async fn compose_by_host_value_gpu() {
  let result = gpu_map(&[0_u32], |_| compose_by_host_value_leaves()).await;
  let data = std140_mixed_data();
  let x = data.c.b.inner.x + data.b_arr.inner[1].inner.inner.x;
  assert_eq!(result, [Vec4::new(data.c.a, x, data.c.c, 4.)]);
}

/// the matrix composed from the host value by `to_shader_node_by_value`, alone and in a struct
#[test]
#[ignore = "bug: the matrix is composed from the scalars instead of the column vectors"]
fn compose_matrix_by_host_value() {
  check_compute(|builder| {
    let zero = builder.global_invocation_id().x();
    keep(Mat3::<f32>::identity().to_shader_node_by_value());
    let leaves = std140_mixed_value_leaves(std140_mixed_data().to_shader_node_by_value(), zero);
    leaves.into_iter().for_each(keep);
  });
}

/// The struct pointer at the u32 offset of the u32 heap (the pointer implementation of the
/// combined buffer), like `u32_heap_ptr` of the harness, but the struct type is registered so the
/// field offsets are known.
fn u32_heap_struct_ptr<T: ShaderSizedValueNodeType>(
  heap: ShaderPtrOf<[u32]>,
  offset: u32,
) -> ShaderPtrOf<T> {
  let mut meta = ShaderU32StructMetaData::new(StructLayoutTarget::Std430);
  meta.register_ty(&MaybeUnsizedValueType::Sized(T::sized_ty()));
  let ptr = U32HeapPtrWithType {
    ptr: U32HeapPtr {
      array: U32HeapHeapSource::Common(heap),
      offset: val(offset),
    },
    ty: ShaderValueSingleType::Sized(T::sized_ty()),
    array_length: None,
    meta: std::sync::Arc::new(parking_lot::RwLock::new(meta)),
  };
  T::create_view_from_raw_ptr(Box::new(ptr))
}

const U32_HEAP_OFFSET: u32 = 4;
const STD140_MIXED_U32_SIZE: u32 = (size_of::<Std140Mixed>() / 4) as u32;

/// The raw buffers are the u32 heap that contains [Std140Mixed] at [U32_HEAP_OFFSET] and the
/// read_write u32 heap, which is written field by field at the same offset and by the whole
/// struct after it.
fn std140_u32_heap_logic(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let zero = builder.global_invocation_id().x();
  let heap = |i: usize| typed_ptr::<[u32]>(&buffers[i]);
  let source = u32_heap_struct_ptr::<Std140Mixed>(heap(0), U32_HEAP_OFFSET);
  let source = as_readonly(&source);
  let by_fields = u32_heap_struct_ptr::<Std140Mixed>(heap(1), U32_HEAP_OFFSET);
  let whole_offset = U32_HEAP_OFFSET + STD140_MIXED_U32_SIZE;
  let whole = u32_heap_struct_ptr::<Std140Mixed>(heap(1), whole_offset);

  let mut r = std140_mixed_ptr_leaves(&source, zero, ArrayAccess::Whole);
  let loaded = source.load();
  r.extend(std140_mixed_value_leaves(loaded, zero));
  std140_mixed_store_by_fields(&by_fields, &source, zero, ArrayAccess::Whole);
  whole.store(loaded);
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&by_fields),
    zero,
    ArrayAccess::Whole,
  ));
  r.extend(std140_mixed_ptr_leaves(
    &as_readonly(&whole),
    zero,
    ArrayAccess::Whole,
  ));
  r
}

fn std140_u32_heap_buffers() -> [RawBuffer; 2] {
  let mut source = vec![0; U32_HEAP_OFFSET as usize * 4];
  source.extend_from_slice(bytes_of(&std140_mixed_data()));
  let target_len = (U32_HEAP_OFFSET + STD140_MIXED_U32_SIZE * 2) as usize * 4;
  [
    RawBuffer {
      ty: <[u32]>::ty(),
      space: BufferSpace::Storage,
      bytes: source,
    },
    RawBuffer {
      ty: <[u32]>::ty(),
      space: BufferSpace::ReadWriteStorage,
      bytes: vec![0; target_len],
    },
  ]
}

/// the padded std140 struct through the u32 heap pointer, its field offsets follow the host
/// layout, loaded and stored by fields and as a whole
#[test]
fn std140_padded_struct_u32_heap() {
  check_raw_buffers(&std140_u32_heap_buffers(), std140_u32_heap_logic);
}

/// the GPU version of [std140_padded_struct_u32_heap], the u32 heap is read and written at the
/// host offsets
#[pollster::test]
async fn std140_padded_struct_u32_heap_gpu() {
  let expect = std140_mixed_cpu_leaves(&std140_mixed_data());
  let result = gpu_run_raw_buffers(
    &std140_u32_heap_buffers(),
    1,
    expect.len() * 4,
    std140_u32_heap_logic,
  )
  .await;
  for (path, leaves) in result.output[0].chunks(expect.len()).enumerate() {
    assert_eq!(leaves, expect, "access path {path}");
  }

  let size = size_of::<Std140Mixed>();
  let start = U32_HEAP_OFFSET as usize * 4;
  for slot in 0..2 {
    let offset = start + slot * size;
    let written: Std140Mixed = pod_read_unaligned(&result.buffers[1][offset..offset + size]);
    assert_eq!(std140_mixed_cpu_leaves(&written), expect, "slot {slot}");
  }
}

/// the fixed size array field is indexed through the u32 heap pointer
#[test]
#[ignore = "bug: the u32 heap pointer does not support the fixed size array index"]
fn u32_heap_fixed_size_array_index() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let ptr = u32_heap_struct_ptr::<Std140Mixed>(fake_storage_buffer::<[u32]>(0), 0);
    keep(ptr.b_arr().index(u).inner().x().load());
    keep(ptr.v4_arr().index(u).load());
  });
}

/// Replace the innermost field of the padded struct, the padded struct is the parameter and the
/// return value.
#[shader_fn]
fn replace_padded_inner(c: Node<Std140C>, x: Node<f32>) -> Node<Std140C> {
  let e = c.expand();
  let b = e.b.expand();
  let inner = ENode::<Std140A> { x }.construct();
  let b = ENode::<Std140B> { inner, ..b }.construct();
  ENode::<Std140C> { b, ..e }.construct()
}

fn replace_padded_inner_leaves(x: Node<f32>) -> Node<Vec4<f32>> {
  let c = replace_padded_inner_fn(val(std140_mixed_data().c), x);
  let b = Std140C::b(c).expand();
  (
    Std140C::a(c),
    b.a.into_f32(),
    Std140A::x(b.inner),
    Std140C::c(c),
  )
    .into()
}

/// the padded struct is passed into and returned from a user function, its fields are accessed
/// and composed in the function
#[test]
fn std140_padded_struct_in_function() {
  check_compute(|builder| {
    keep(replace_padded_inner_leaves(runtime_values(builder).f));
  });
}

/// the GPU version of [std140_padded_struct_in_function]
#[pollster::test]
async fn std140_padded_struct_in_function_gpu() {
  let input = [0.5, 1.5];
  let result = gpu_map(&input, replace_padded_inner_leaves).await;
  let c = std140_mixed_data().c;
  let expect: Vec<_> = input
    .iter()
    .map(|x| Vec4::new(c.a, c.b.a as f32, *x, c.c))
    .collect();
  assert_eq!(result, expect);
}

/// compose the padded structs at each nesting level from runtime values, access the fields of the
/// composed value and of the local variable by runtime index, and write the fields through the
/// local pointer
#[test]
fn std140_padded_struct_compose() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    let a = ENode::<Std140A> { x: f }.construct();
    let b = ENode::<Std140B> { a: u, inner: a }.construct();
    let c = ENode::<Std140C> { a: f, b, c: f }.construct();
    keep(Std140C::b(c).expand().inner.expand().x + Std140C::c(c));

    let pair = ENode::<Std140Pair> { a: u, b: f.splat() }.construct();
    let mixed = ENode::<Std140Mixed> {
      c,
      pair,
      head: f,
      tail: u,
      ..zeroed_val::<Std140Mixed>().expand()
    }
    .construct();
    keep(Std140Pair::b(Std140Mixed::pair(mixed)));

    let local = mixed.make_local_var();
    local.b_arr().index(u).inner().x().store(f);
    local.b_arr().index(u).store(b);
    local.c().b().store(Std140C::b(c));
    local.v3_arr().index(u).y().store(f);
    local.m3().index(u).store(f.splat());
    keep(local.b_arr().index(u).inner().x().load());
    keep(local.c().b().inner().load());
    keep(local.load());

    let array = make_local_var::<[Std140C; 4]>();
    array.index(u).b().inner().x().store(f);
    array.index(u + val(1)).store(c);
    keep(array.index(u).b().a().load());
  });
}

/// the constants of the padded structs, they are composed in the global expressions with the
/// padding members: nested structs, arrays of padded struct, the constant as a compose component
/// and as a stored value
#[test]
fn std140_padded_struct_constants() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let data = std140_mixed_data();

    keep(Std140A::x(val(std140_a(1.))));
    keep(Std140B::inner(val(std140_b(1, 2.))));
    keep(Std140C::b(val(data.c)).expand().inner);
    keep(Std140C::c(global_const_val(data.c)));
    keep(Std140Mixed::m3(val(data)));

    let array = val(data.b_arr.into_shader_ty()).make_local_var();
    keep(array.index(u).inner().x().load());

    let c = ENode::<Std140C> {
      b: val(std140_b(2, 3.)),
      ..val(data.c).expand()
    }
    .construct();
    keep(c);

    let local = make_local_var::<Std140Mixed>();
    local.store(val(data));
    local.c().store(global_const_val(data.c));
    local.b_arr().index(u).store(val(std140_b(4, 5.)));
    keep(local.load());
  });
}

/// The std430 struct has the same layout as the natural WGSL layout, but the rust layout still
/// has implicit paddings, like the scalar before vec3.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy)]
pub struct Std430Inner {
  pub a: f32,
  pub v: Vec3<f32>,
}

/// The nested struct, arrays of vector and struct, the matrices and the scalar after vec3.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy)]
pub struct Std430Item {
  pub id: u32,
  pub inner: Std430Inner,
  pub v2: Vec2<f32>,
  pub m3x2: Mat3x2<f32>,
  pub u2_arr: [Vec2<u32>; 3],
  pub inner_arr: [Std430Inner; 2],
  pub v3: Vec3<f32>,
  pub flag: Bool,
  pub m4: Mat4<f32>,
  pub tail: f32,
}

pub fn std430_inner(a: f32, v: Vec3<f32>) -> Std430Inner {
  Std430Inner {
    a,
    v,
    ..Zeroable::zeroed()
  }
}

/// Every leaf value is distinct among the items.
pub fn std430_item(k: u32) -> Std430Item {
  let mut n = k as f32 * 100.;
  let mut next = || {
    n += 1.;
    n
  };
  let next_vec3 = |next: &mut dyn FnMut() -> f32| Vec3::new(next(), next(), next());
  Std430Item {
    id: k + 1,
    inner: std430_inner(next(), next_vec3(&mut next)),
    v2: Vec2::new(next(), next()),
    m3x2: Mat3x2::from([next(), next(), next(), next(), next(), next()]),
    u2_arr: [
      Vec2::new(k, 10 + k),
      Vec2::new(20 + k, 30 + k),
      Vec2::new(40, 50),
    ],
    inner_arr: [
      std430_inner(next(), next_vec3(&mut next)),
      std430_inner(next(), next_vec3(&mut next)),
    ],
    v3: next_vec3(&mut next),
    flag: k.is_multiple_of(2).into(),
    m4: Mat4::from(std::array::from_fn(|_| next())),
    tail: next(),
    ..Zeroable::zeroed()
  }
}

pub fn std430_inner_cpu_leaves(v: &Std430Inner) -> [f32; 4] {
  [v.a, v.v.x, v.v.y, v.v.z]
}

pub fn std430_item_cpu_leaves(v: &Std430Item) -> Vec<f32> {
  let mut r = vec![v.id as f32];
  r.extend(std430_inner_cpu_leaves(&v.inner));
  r.extend([v.v2.x, v.v2.y]);
  let m3x2: [f32; 6] = v.m3x2.into();
  r.extend(m3x2);
  for x in v.u2_arr {
    r.extend([x.x as f32, x.y as f32]);
  }
  for x in &v.inner_arr {
    r.extend(std430_inner_cpu_leaves(x));
  }
  r.extend([v.v3.x, v.v3.y, v.v3.z, bool::from(v.flag) as u32 as f32]);
  let m4: [f32; 16] = v.m4.into();
  r.extend(m4);
  r.push(v.tail);
  r
}

fn std430_inner_transform(v: Node<Std430Inner>, negate: bool) -> Node<Std430Inner> {
  let v = v.expand();
  let a = if negate { -v.a } else { v.a + val(1.) };
  ENode::<Std430Inner> { a, v: v.v.zxy() }.construct()
}

fn std430_inner_transform_cpu(v: &Std430Inner, negate: bool) -> Std430Inner {
  let a = if negate { -v.a } else { v.a + 1. };
  std430_inner(a, Vec3::new(v.v.z, v.v.x, v.v.y))
}

/// Transform every field, the arrays are reversed through the local variables.
fn std430_item_transform(v: Node<Std430Item>) -> Node<Std430Item> {
  let e = v.expand();

  let u2_arr = e.u2_arr.make_local_var();
  let reversed = make_local_var::<[Vec2<u32>; 3]>();
  for i in 0..3 {
    let item = u2_arr.index(val(i)).load() + val(1_u32);
    reversed.index(val(2 - i)).store(item);
  }

  let inner_arr = e.inner_arr.make_local_var();
  let swapped = make_local_var::<[Std430Inner; 2]>();
  for i in 0..2 {
    let item = std430_inner_transform(inner_arr.index(val(i)).load(), true);
    swapped.index(val(1 - i)).store(item);
  }

  ENode::<Std430Item> {
    id: e.id * val(2) + val(1),
    inner: std430_inner_transform(e.inner, false),
    v2: e.v2 * val(2.),
    m3x2: e.m3x2 * val(2.),
    u2_arr: reversed.load(),
    inner_arr: swapped.load(),
    v3: e.v3 + e.tail,
    flag: e.flag.into_bool().not().into_big_bool(),
    m4: e.m4.transpose(),
    tail: e.tail + e.id.into_f32(),
  }
  .construct()
}

fn std430_item_transform_cpu(v: &Std430Item) -> Std430Item {
  let m3x2: [f32; 6] = v.m3x2.into();
  let m4: [f32; 16] = v.m4.into();
  let [a, b, c] = v.u2_arr.map(|x| Vec2::new(x.x + 1, x.y + 1));
  Std430Item {
    id: v.id * 2 + 1,
    inner: std430_inner_transform_cpu(&v.inner, false),
    v2: Vec2::new(v.v2.x * 2., v.v2.y * 2.),
    m3x2: Mat3x2::from(m3x2.map(|x| x * 2.)),
    u2_arr: [c, b, a],
    inner_arr: [
      std430_inner_transform_cpu(&v.inner_arr[1], true),
      std430_inner_transform_cpu(&v.inner_arr[0], true),
    ],
    v3: Vec3::new(v.v3.x + v.tail, v.v3.y + v.tail, v.v3.z + v.tail),
    flag: (!bool::from(v.flag)).into(),
    m4: Mat4::from(std::array::from_fn(|i| m4[(i % 4) * 4 + i / 4])),
    tail: v.tail + v.id as f32,
    ..Zeroable::zeroed()
  }
}

/// the std430 struct through the storage buffers and the local variables, and the composing
#[test]
fn std430_struct_access() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let items = fake_storage_buffer::<[Std430Item]>(0);
    let item = items.index(u);

    keep(item.inner().v().load());
    keep(item.inner_arr().index(u).v().z().load());
    keep(item.u2_arr().index(u).load());
    keep(item.m3x2().index(u).load());
    item.inner_arr().index(u).a().store(f);
    item.m4().w().store(f.splat());
    item.store(std430_item_transform(item.load()));

    let readonly = items.into_readonly_view();
    keep(readonly.index(u).inner_arr().index(u).a().load());

    let local = readonly.index(u).load().make_local_var();
    local.inner().store(val(std430_inner(1., Vec3::one())));
    keep(local.inner_arr().load());
  });
}

/// the std430 structs are transformed on the GPU, the input and output layouts agree with the
/// host layout
#[pollster::test]
async fn std430_struct_gpu_transform() {
  let input = [std430_item(0), std430_item(1), std430_item(2)];
  let result = gpu_map(&input, std430_item_transform).await;
  for (result, input) in result.iter().zip(&input) {
    assert_eq!(
      std430_item_cpu_leaves(result),
      std430_item_cpu_leaves(&std430_item_transform_cpu(input))
    );
  }
}

/// the std430 struct constants (the nested struct, arrays of vector and struct, the matrices) are
/// selected on the GPU, they agree with the host layout
#[pollster::test]
async fn std430_struct_gpu_constants() {
  let items = [std430_item(0), std430_item(1), std430_item(2)];
  let result = gpu_map(&[0_u32, 1, 2], move |i| {
    i.equals(val(0)).select_branched(
      || val(items[0]),
      || {
        i.equals(val(1))
          .select_branched(|| val(items[1]), || val(items[2]))
      },
    )
  })
  .await;
  for (result, item) in result.iter().zip(&items) {
    assert_eq!(std430_item_cpu_leaves(result), std430_item_cpu_leaves(item));
  }
}

/// the GPU version of the named constant (not inlined into the function) of the std430 struct
#[pollster::test]
#[ignore = "bug: naga msl output of the named struct constant with a vec3 after a gap fails to compile on Metal"]
async fn std430_struct_gpu_named_constant() {
  let item = std430_item(0);
  let result = gpu_map(&[0_u32], move |_| global_const_val(item)).await;
  assert_eq!(
    std430_item_cpu_leaves(&result[0]),
    std430_item_cpu_leaves(&item)
  );
}

const MAT2X4_ARRAY: [Mat2x4<f32>; 2] = [
  Mat2x4 {
    a1: 1.,
    a2: 2.,
    a3: 3.,
    a4: 4.,
    b1: 5.,
    b2: 6.,
    b3: 7.,
    b4: 8.,
  },
  Mat2x4 {
    a1: 9.,
    a2: 10.,
    a3: 11.,
    a4: 12.,
    b1: 13.,
    b2: 14.,
    b3: 15.,
    b4: 16.,
  },
];

const NESTED_ARRAY: [[Vec2<f32>; 2]; 2] = [
  [Vec2 { x: 1., y: 2. }, Vec2 { x: 3., y: 4. }],
  [Vec2 { x: 5., y: 6. }, Vec2 { x: 7., y: 8. }],
];

/// the constants of every matrix type, the arrays of matrices and the nested arrays
#[test]
fn matrix_and_array_constants() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    keep(val(Mat2::<f32>::identity()) * f);
    keep(val(Mat3::<f32>::identity()) * f);
    keep(val(Mat4::<f32>::identity()) * f);
    keep(val(Mat2x3::<f32>::default()) * f);
    keep(val(Mat2x4::<f32>::default()) * f);
    keep(val(Mat3x2::<f32>::default()) * f);
    keep(val(Mat3x4::<f32>::default()) * f);
    keep(val(Mat4x2::<f32>::default()) * f);
    keep(val(Mat4x3::<f32>::default()) * f);

    keep(val(MAT2X4_ARRAY).make_local_var().index(u).y().load());
    keep(
      global_const_val(MAT2X4_ARRAY)
        .make_local_var()
        .index(u)
        .load(),
    );
    keep(val(NESTED_ARRAY).make_local_var().index(u).index(u).load());
    keep(val([1_u32, 2, 3, 4]).index(u));
    keep(Node::<[f32; 3]>::from_array([1., 2., 3.]).index(u));
    keep(
      val([std430_inner(1., Vec3::one()); 3])
        .make_local_var()
        .index(u)
        .v()
        .load(),
    );
  });
}

/// the arrays of matrices and the nested arrays constants are indexed on the GPU
#[pollster::test]
async fn matrix_and_array_constants_gpu() {
  let result = gpu_map(&[0_u32, 1], |i| {
    let matrices = val(MAT2X4_ARRAY).make_local_var();
    let nested = val(NESTED_ARRAY).make_local_var();
    let vec2 = nested.index(i).index(val(1) - i).load();
    matrices.index(i).y().load() + vec2.xyxy()
  })
  .await;
  let expect: Vec<_> = (0..2)
    .map(|i| {
      let m: [f32; 8] = MAT2X4_ARRAY[i].into();
      let v = NESTED_ARRAY[i][1 - i];
      Vec4::new(m[4] + v.x, m[5] + v.y, m[6] + v.x, m[7] + v.y)
    })
    .collect();
  assert_eq!(result, expect);
}

/// A hand written host type layout which has larger gaps than the std layout rules: f32 at 0,
/// u32 at 16, vec2<f32> at 40, vec4<f32> at 64 and the size is 96.
fn sparse_struct_ty() -> ShaderSizedValueType {
  let meta = ShaderStructMetaInfo::new("SparseHostLayout")
    .add_field::<f32>("a")
    .add_field::<u32>("b")
    .add_field::<Vec2<f32>>("c")
    .add_field::<Vec4<f32>>("d")
    .with_host_layout(ShaderStructHostLayout {
      target: ShaderHostLayoutTarget::Std430,
      field_offsets: vec![0, 16, 40, 64],
      size: 96,
    });
  ShaderSizedValueType::Struct(meta)
}

const SPARSE_U32_OFFSETS: [usize; 4] = [0, 4, 10, 16];
const SPARSE_U32_SIZE: usize = 24;

/// the host bytes of the sparse struct, the field values are `k * 10 + n`
fn sparse_struct_host_data(k: u32) -> [u32; SPARSE_U32_SIZE] {
  let base = k as f32 * 10.;
  let mut data = [0; SPARSE_U32_SIZE];
  data[SPARSE_U32_OFFSETS[0]] = (base + 1.).to_bits();
  data[SPARSE_U32_OFFSETS[1]] = k * 10 + 2;
  for i in 0..2 {
    data[SPARSE_U32_OFFSETS[2] + i] = (base + 3. + i as f32).to_bits();
  }
  for i in 0..4 {
    data[SPARSE_U32_OFFSETS[3] + i] = (base + 5. + i as f32).to_bits();
  }
  data
}

/// the leaves of the sparse struct at the pointer
fn sparse_struct_ptr_leaves(ptr: &BoxedShaderPtr) -> Vec<Node<f32>> {
  let c = typed_readonly_ptr::<Vec2<f32>>(&ptr.field_index(2)).load();
  let d = typed_readonly_ptr::<Vec4<f32>>(&ptr.field_index(3)).load();
  let mut r = vec![
    typed_readonly_ptr::<f32>(&ptr.field_index(0)).load(),
    typed_readonly_ptr::<u32>(&ptr.field_index(1))
      .load()
      .into_f32(),
  ];
  vec2_leaves(&mut r, c);
  vec4_leaves(&mut r, d);
  r
}

/// the leaves of the sparse struct value
fn sparse_struct_value_leaves(value: ShaderNodeRawHandle) -> Vec<Node<f32>> {
  let field = |i| unsafe { index_access_field(value, i) };
  let mut r = unsafe { vec![field(0).into_node(), field(1).into_node::<u32>().into_f32()] };
  vec2_leaves(&mut r, unsafe { field(2).into_node() });
  vec4_leaves(&mut r, unsafe { field(3).into_node() });
  r
}

fn sparse_struct_cpu_leaves(data: &[u32]) -> Vec<f32> {
  let f = |i: usize| f32::from_bits(data[i]);
  let [a, b, c, d] = SPARSE_U32_OFFSETS;
  vec![
    f(a),
    data[b] as f32,
    f(c),
    f(c + 1),
    f(d),
    f(d + 1),
    f(d + 2),
    f(d + 3),
  ]
}

/// The raw buffers are the readonly storage of the sparse struct array and the read_write storage
/// of the sparse struct, the read_write storage is written by the composed value of the array
/// item 1, then its field c is written through the pointer.
fn sparse_struct_logic(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let zero = builder.global_invocation_id().x();
  let array = &buffers[0];
  let output = &buffers[1];
  let item = |i: u32| array.field_array_index(zero + val(i));

  let mut r = vec![array.array_length().into_f32()];
  r.extend(sparse_struct_ptr_leaves(&item(0)));
  r.extend(sparse_struct_ptr_leaves(&item(1)));
  let loaded = item(1).load();
  r.extend(sparse_struct_value_leaves(loaded));

  let field = |i| unsafe { index_access_field(loaded, i) };
  let composed = ShaderNodeExpr::Compose {
    target: sparse_struct_ty(),
    parameters: vec![field(0), field(1), field(2), field(3)],
  }
  .insert_api_raw();
  output.store(composed);
  let c = typed_ptr::<Vec2<f32>>(&output.field_index(2));
  c.store(c.load() * val(2.));
  r.extend(sparse_struct_value_leaves(composed));
  r
}

fn sparse_struct_buffers() -> [RawBuffer; 2] {
  let mut array = sparse_struct_host_data(0).to_vec();
  array.extend(sparse_struct_host_data(1));
  let array = cast_slice(&array).to_vec();
  let empty = vec![0; SPARSE_U32_SIZE * 4];
  let element = Box::new(sparse_struct_ty());
  [
    RawBuffer {
      ty: ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(element),
      )),
      space: BufferSpace::Storage,
      bytes: array,
    },
    RawBuffer {
      ty: ShaderValueType::Single(ShaderValueSingleType::Sized(sparse_struct_ty())),
      space: BufferSpace::ReadWriteStorage,
      bytes: empty,
    },
  ]
}

/// the hand written host layout attached by `with_host_layout`, the naga backend fills the gaps by
/// padding members, in the runtime sized array, the value, the composing and the field pointer
#[test]
fn hand_written_host_layout() {
  check_raw_buffers(&sparse_struct_buffers(), sparse_struct_logic);
}

/// the GPU version of [hand_written_host_layout], the fields are read and written at the host
/// offsets
#[pollster::test]
async fn hand_written_host_layout_gpu() {
  let result =
    gpu_run_raw_buffers(&sparse_struct_buffers(), 1, 1 + 8 * 4, sparse_struct_logic).await;
  let item0 = sparse_struct_host_data(0);
  let item1 = sparse_struct_host_data(1);
  let mut expect = vec![2.];
  expect.extend(sparse_struct_cpu_leaves(&item0));
  expect.extend(sparse_struct_cpu_leaves(&item1));
  expect.extend(sparse_struct_cpu_leaves(&item1));
  expect.extend(sparse_struct_cpu_leaves(&item1));
  assert_eq!(result.output[0], expect);

  let written = read_pods::<u32>(&result.buffers[1]);
  let mut expect = item1;
  for i in 0..2 {
    let c = &mut expect[SPARSE_U32_OFFSETS[2] + i];
    *c = (f32::from_bits(*c) * 2.).to_bits();
  }
  assert_eq!(written, expect);
}

fn zeroed_struct(meta: ShaderStructMetaInfo) {
  build_compute(|_| {
    ShaderNodeExpr::Zeroed {
      target: ShaderSizedValueType::Struct(meta),
    }
    .insert_api_raw();
  });
}

/// the host field offset smaller than the natural offset is rejected
#[test]
#[should_panic(expected = "host layout of struct `BadOffset` field `b` is invalid for WGSL")]
fn host_layout_offset_smaller_than_natural() {
  zeroed_struct(
    ShaderStructMetaInfo::new("BadOffset")
      .add_field::<f32>("a")
      .add_field::<Vec4<f32>>("b")
      .with_host_layout(ShaderStructHostLayout {
        target: ShaderHostLayoutTarget::Std430,
        field_offsets: vec![0, 4],
        size: 32,
      }),
  );
}

/// the host field offset that is not aligned to the field alignment is rejected
#[test]
#[should_panic(expected = "host layout of struct `BadAlign` field `b` is invalid for WGSL")]
fn host_layout_offset_not_aligned() {
  zeroed_struct(
    ShaderStructMetaInfo::new("BadAlign")
      .add_field::<f32>("a")
      .add_field::<Vec2<f32>>("b")
      .with_host_layout(ShaderStructHostLayout {
        target: ShaderHostLayoutTarget::Std430,
        field_offsets: vec![0, 12],
        size: 32,
      }),
  );
}

/// the host struct size smaller than the natural size or not aligned is rejected
#[test]
#[should_panic(expected = "host layout of struct `BadSize` is invalid for WGSL")]
fn host_layout_size_invalid() {
  zeroed_struct(
    ShaderStructMetaInfo::new("BadSize")
      .add_field::<Vec4<f32>>("a")
      .add_field::<f32>("b")
      .with_host_layout(ShaderStructHostLayout {
        target: ShaderHostLayoutTarget::Std140,
        field_offsets: vec![0, 16],
        size: 40,
      }),
  );
}
