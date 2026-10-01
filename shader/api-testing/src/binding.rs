use std::sync::Arc;

use rendiation_shader_api::*;
use rendiation_webgpu::*;

use crate::harness::*;
use crate::layout::*;

/// The buffer address space of [RawBuffer].
#[derive(Clone, Copy)]
pub enum BufferSpace {
  Uniform,
  Storage,
  ReadWriteStorage,
}

/// A buffer binding of any shader type in any buffer address space with the host bytes.
///
/// The typed containers only accept the std140 type as uniform and the std430 type as storage,
/// the raw buffer is used to bind the other combinations (for example the std140 struct in the
/// storage buffer), and the types that have no rust type (the unsized struct and the hand written
/// host layout).
pub struct RawBuffer {
  pub ty: ShaderValueType,
  pub space: BufferSpace,
  pub bytes: Vec<u8>,
}

impl RawBuffer {
  fn desc(&self) -> ShaderBindingDescriptor {
    buffer_binding_desc(self.ty.clone(), self.space)
  }
}

fn buffer_binding_desc(ty: ShaderValueType, space: BufferSpace) -> ShaderBindingDescriptor {
  ShaderBindingDescriptor {
    should_as_storage_buffer_if_is_buffer_like: !matches!(space, BufferSpace::Uniform),
    ty,
    writeable_if_storage: matches!(space, BufferSpace::ReadWriteStorage),
    has_dynamic_offset: false,
  }
}

/// Create a buffer binding of `T` in the address space without any GPU resource container.
pub fn fake_buffer<T: ShaderNodeType + ?Sized>(
  entry_index: usize,
  space: BufferSpace,
) -> BoxedShaderPtr {
  fake_buffer_binding(entry_index, buffer_binding_desc(T::ty(), space))
}

/// The logic of the raw buffer test, it gets the pointers of the raw buffers in order and returns
/// the values written to the f32 output of the invocation.
pub type RawBufferLogic = fn(&ShaderComputePipelineBuilder, &[BoxedShaderPtr]) -> Vec<Node<f32>>;

/// Build the raw buffer test by the fake bindings and validate it, the output is the next binding
/// after the raw buffers.
pub fn check_raw_buffers(buffers: &[RawBuffer], logic: RawBufferLogic) {
  check_compute(|builder| {
    let ptrs: Vec<_> = buffers
      .iter()
      .enumerate()
      .map(|(i, buffer)| fake_buffer_binding(i, buffer.desc()))
      .collect();
    let output = fake_storage_buffer::<[f32]>(buffers.len());
    for (i, v) in logic(builder, &ptrs).into_iter().enumerate() {
      output.index(val(i as u32)).store(v);
    }
  });
}

fn fake_buffer_binding(entry_index: usize, desc: ShaderBindingDescriptor) -> BoxedShaderPtr {
  let handle = ShaderInputNode::Binding {
    desc,
    bindgroup_index: 0,
    entry_index,
  }
  .insert_api_raw();
  Box::new(handle)
}

struct RawGPUBuffer {
  view: GPUBufferResourceView,
  desc: ShaderBindingDescriptor,
}

impl ShaderBindingProvider for RawGPUBuffer {
  type Node = AnyType;
  type ShaderInstance = BoxedShaderPtr;
  fn create_instance(&self, node: Node<AnyType>) -> BoxedShaderPtr {
    Box::new(node.handle())
  }
  fn binding_desc(&self) -> ShaderBindingDescriptor {
    self.desc.clone()
  }
}

impl CacheAbleBindingSource for RawGPUBuffer {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.view.get_binding_build_source()
  }
}

/// The result of [gpu_run_raw_buffers].
pub struct RawBufferResult {
  /// the outputs of each invocation, in the local invocation index order
  pub output: Vec<Vec<f32>>,
  /// the bytes of each raw buffer after the execution
  pub buffers: Vec<Vec<u8>>,
}

/// Run the raw buffer test in one workgroup on the GPU, each invocation returns `output_len`
/// values. A GPU adapter is required.
pub async fn gpu_run_raw_buffers(
  buffers: &[RawBuffer],
  workgroup_size: u32,
  output_len: usize,
  logic: RawBufferLogic,
) -> RawBufferResult {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let raw_buffers: Vec<_> = buffers
    .iter()
    .map(|buffer| {
      let space_usage = match buffer.space {
        BufferSpace::Uniform => BufferUsages::UNIFORM,
        _ => BufferUsages::STORAGE,
      };
      let usage = space_usage | BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
      let init = BufferInit::WithInit(&buffer.bytes);
      let desc = GPUBufferDescriptor {
        size: init.size(),
        usage,
      };
      let raw = GPUBuffer::create(&gpu.device, None, init, usage);
      RawGPUBuffer {
        view: GPUBufferResource::create_with_raw(raw, desc, &gpu.device).create_default_view(),
        desc: buffer.desc(),
      }
    })
    .collect();

  let output = create_gpu_read_write_storage::<[f32]>(
    ZeroedArrayByArrayLength(workgroup_size as usize * output_len),
    &gpu,
    "raw buffer test output",
  );

  let hasher = shader_hasher_from_marker_ty!(RawBufferTest).with_hash(logic as usize);
  let pipeline = gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(workgroup_size);
      let ptrs: Vec<_> = raw_buffers.iter().map(|b| builder.bind_by(b)).collect();
      let output_ptr = builder.bind_by(&output);
      let values = logic(&builder, &ptrs);
      assert_eq!(values.len(), output_len, "unexpected output count");
      let base = builder.local_invocation_index() * val(output_len as u32);
      for (i, v) in values.into_iter().enumerate() {
        output_ptr.index(base + val(i as u32)).store(v);
      }
      builder
    });

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    let mut binding = BindingBuilder::default();
    for buffer in &raw_buffers {
      binding.bind(buffer);
    }
    binding
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);
  });

  let output = encoder.read_buffer(&gpu.device, &output);
  let buffers: Vec<_> = raw_buffers
    .iter()
    .map(|b| encoder.read_buffer_bytes(&gpu.device, &b.view))
    .collect();
  gpu.submit_encoder(encoder);

  let output = <[f32]>::from_bytes_into_boxed(&output.await.unwrap().read_raw()).into_vec();
  let mut buffer_bytes = Vec::with_capacity(buffers.len());
  for b in buffers {
    buffer_bytes.push(b.await.unwrap());
  }
  RawBufferResult {
    output: output.chunks(output_len).map(|c| c.to_vec()).collect(),
    buffers: buffer_bytes,
  }
}

/// Decode the host values from the bytes read back from the GPU.
pub fn read_pods<T: Pod>(bytes: &[u8]) -> Vec<T> {
  bytes
    .chunks_exact(size_of::<T>())
    .map(pod_read_unaligned)
    .collect()
}

/// A private address space variable, the EDSL has no dedicated api for it yet.
pub fn private_var<T: ShaderSizedValueNodeType>() -> ShaderPtrOf<T> {
  let handle = ShaderInputNode::Private { ty: T::sized_ty() }.insert_api_raw();
  T::create_view_from_raw_ptr(Box::new(handle))
}

/// The readonly view of the pointer.
pub fn as_readonly<P: SizedShaderPtrView>(ptr: &P) -> ShaderReadonlyPtrOf<P::Node> {
  P::Node::create_readonly_view_from_raw_ptr(ptr.raw().clone())
}

/// The typed view of the raw pointer.
pub fn typed_ptr<T: ShaderAbstractPtrAccess + ?Sized>(ptr: &BoxedShaderPtr) -> ShaderPtrOf<T> {
  T::create_view_from_raw_ptr(ptr.clone())
}

/// The typed readonly view of the raw pointer.
pub fn typed_readonly_ptr<T: ShaderAbstractPtrAccess + ?Sized>(
  ptr: &BoxedShaderPtr,
) -> ShaderReadonlyPtrOf<T> {
  T::create_readonly_view_from_raw_ptr(ptr.clone())
}

/// the non struct types and the structs in each buffer address space: scalar, vector, matrix,
/// fixed size array, the array of padded struct and the std140 and std430 structs, loaded, indexed
/// and written
#[test]
fn buffer_address_spaces() {
  use BufferSpace::*;
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    keep(typed_readonly_ptr::<u32>(&fake_buffer::<u32>(0, Uniform)).load());
    keep(typed_readonly_ptr::<Vec4<f32>>(&fake_buffer::<Vec4<f32>>(1, Uniform)).load());
    let matrix = typed_readonly_ptr::<Mat4<f32>>(&fake_buffer::<Mat4<f32>>(2, Uniform));
    keep(matrix.w().load() + matrix.index(u).load());
    let array = fake_buffer::<[Vec4<f32>; 4]>(3, Uniform);
    keep(typed_readonly_ptr::<[Vec4<f32>; 4]>(&array).index(u).load());
    let pair = typed_readonly_ptr::<Std140Pair>(&fake_buffer::<Std140Pair>(4, Uniform));
    keep(pair.b().load());
    let padded = fake_buffer::<[Std140B; 4]>(5, Uniform);
    let padded = typed_readonly_ptr::<[Std140B; 4]>(&padded);
    keep(padded.index(u).inner().x().load());
    keep(padded.load());

    let array = fake_buffer::<[f32; 4]>(6, Storage);
    keep(typed_readonly_ptr::<[f32; 4]>(&array).index(u).load());
    let matrix = fake_buffer::<Mat3x2<f32>>(7, Storage);
    keep(typed_readonly_ptr::<Mat3x2<f32>>(&matrix).index(u).load());
    let item = typed_readonly_ptr::<Std430Item>(&fake_buffer::<Std430Item>(8, Storage));
    keep(item.inner_arr().index(u).v().load());
    keep(item.load());

    typed_ptr::<u32>(&fake_buffer::<u32>(9, ReadWriteStorage)).store(u);
    let matrix = typed_ptr::<Mat4<f32>>(&fake_buffer::<Mat4<f32>>(10, ReadWriteStorage));
    matrix.w().store(f.splat());
    matrix.index(u).y().store(f);
    let array = typed_ptr::<[f32; 4]>(&fake_buffer::<[f32; 4]>(11, ReadWriteStorage));
    array.index(u).store(f);
    keep(array.load());
    let item = typed_ptr::<Std430Item>(&fake_buffer::<Std430Item>(12, ReadWriteStorage));
    item.inner_arr().index(u).a().store(f);
    item.u2_arr().index(u).x().store(u);
    item.store(item.load());
    keep(as_readonly(&item).m4().load());
  });
}

/// the runtime sized array of structs, nested arrays, matrices and atomics, the array length and
/// the iteration
#[test]
fn runtime_sized_array() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    let items = fake_storage_buffer::<[Std430Item]>(0);
    keep(items.array_length());
    let item = items.index(u);
    item.inner_arr().index(u).v().store(f.splat());
    item.m4().index(u).store(f.splat());
    item.store(zeroed_val());
    let readonly = items.into_readonly_view();
    keep(readonly.array_length());
    keep(readonly.index(u).inner().load());

    let inners =
      typed_readonly_ptr::<[Std430Inner]>(&fake_buffer::<[Std430Inner]>(1, BufferSpace::Storage));
    keep(inners.array_length());
    keep(inners.index(u).a().load());
    keep(
      inners
        .into_shader_iter()
        .map(|(_, item)| item.a().load())
        .sum(),
    );

    let atomics = fake_storage_buffer::<[DeviceAtomic<u32>]>(2);
    keep(atomics.array_length());
    keep(atomics.index(u).atomic_add(val(1)));

    let arrays = fake_storage_buffer::<[[Vec4<f32>; 2]]>(3);
    arrays.index(u).index(u).store(f.splat());
    keep(arrays.array_length());

    let matrices = fake_storage_buffer::<[Mat4<f32>]>(4);
    matrices.index(u).w().store(f.splat());
    keep(matrices.index(u).load());
  });
}

/// The raw buffers are the readonly storage of [Std430Inner] array with [RUNTIME_ARRAY_LEN]
/// items, the read_write storage of the same length and the atomic counters.
fn runtime_sized_array_logic(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let id = builder.local_invocation_index();
  let input = typed_readonly_ptr::<[Std430Inner]>(&buffers[0]);
  let output = typed_ptr::<[Std430Inner]>(&buffers[1]);
  let counters = typed_ptr::<[DeviceAtomic<u32>]>(&buffers[2]);

  let len = input.array_length();
  let reversed = input.index(len - val(1) - id).load().expand();
  output.index(id).store(
    ENode::<Std430Inner> {
      a: reversed.a * val(2.),
      v: reversed.v.zyx(),
    }
    .construct(),
  );
  counters.index(val(0)).atomic_add(id + val(1));
  counters.index(val(1)).atomic_max(id);

  let item = input.index(id).load().expand();
  let sum = input
    .into_shader_iter()
    .map(|(_, item)| item.a().load())
    .sum();
  vec![
    len.into_f32(),
    output.array_length().into_f32(),
    counters.array_length().into_f32(),
    item.a,
    item.v.z(),
    sum,
  ]
}

const RUNTIME_ARRAY_LEN: usize = 5;

fn runtime_sized_array_input() -> Vec<Std430Inner> {
  (0..RUNTIME_ARRAY_LEN)
    .map(|i| {
      let i = i as f32;
      std430_inner(i + 1., Vec3::new(i * 10., i * 10. + 1., i * 10. + 2.))
    })
    .collect()
}

fn runtime_sized_array_buffers() -> [RawBuffer; 3] {
  let input = runtime_sized_array_input();
  [
    RawBuffer {
      ty: <[Std430Inner]>::ty(),
      space: BufferSpace::Storage,
      bytes: cast_slice(&input).to_vec(),
    },
    RawBuffer {
      ty: <[Std430Inner]>::ty(),
      space: BufferSpace::ReadWriteStorage,
      bytes: vec![0; size_of::<Std430Inner>() * RUNTIME_ARRAY_LEN],
    },
    RawBuffer {
      ty: <[DeviceAtomic<u32>]>::ty(),
      space: BufferSpace::ReadWriteStorage,
      bytes: vec![0; 8],
    },
  ]
}

/// the runtime sized array of the std430 struct read, written, iterated and the array length
#[test]
fn runtime_sized_array_of_struct() {
  check_raw_buffers(&runtime_sized_array_buffers(), runtime_sized_array_logic);
}

/// the GPU version of [runtime_sized_array_of_struct], one invocation for each item
#[pollster::test]
async fn runtime_sized_array_of_struct_gpu() {
  let result = gpu_run_raw_buffers(
    &runtime_sized_array_buffers(),
    RUNTIME_ARRAY_LEN as u32,
    6,
    runtime_sized_array_logic,
  )
  .await;

  let input = runtime_sized_array_input();
  let sum = input.iter().map(|item| item.a).sum();
  for (id, output) in result.output.iter().enumerate() {
    let len = RUNTIME_ARRAY_LEN as f32;
    let expect = [len, len, 2., input[id].a, input[id].v.z, sum];
    assert_eq!(output, &expect, "invocation {id}");
  }

  let written = read_pods::<Std430Inner>(&result.buffers[1]);
  for (id, written) in written.iter().enumerate() {
    let reversed = input[RUNTIME_ARRAY_LEN - 1 - id];
    let v = reversed.v;
    let expect = [reversed.a * 2., v.z, v.y, v.x];
    assert_eq!(std430_inner_cpu_leaves(written), expect, "item {id}");
  }

  let counters: [u32; 2] = pod_read_unaligned(&result.buffers[2]);
  let n = RUNTIME_ARRAY_LEN as u32;
  assert_eq!(counters, [n * (n + 1) / 2, n - 1]);
}

/// The sized fields of [unsized_struct_ty].
#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy)]
pub struct UnsizedHead {
  pub count: u32,
  pub scale: f32,
  pub inner: Std430Inner,
}

/// The struct with the trailing runtime sized array, the EDSL has no rust type for it.
fn unsized_struct_ty() -> ShaderValueType {
  static META: std::sync::LazyLock<ShaderUnSizedStructMetaInfo> =
    std::sync::LazyLock::new(|| ShaderUnSizedStructMetaInfo {
      name: "UnsizedItems".into(),
      sized_fields: UnsizedHead::meta_info().fields,
      last_dynamic_array_field: ("items".into(), Box::new(Std430Inner::sized_ty())),
    });
  ShaderValueType::Single(ShaderValueSingleType::Unsized(
    ShaderUnSizedValueType::UnsizedStruct(&META),
  ))
}

/// The raw buffers are the readonly and read_write storage of [unsized_struct_ty], the read_write
/// one is written from the readonly one.
fn unsized_struct_logic(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let zero = builder.global_invocation_id().x();
  let (input, output) = (&buffers[0], &buffers[1]);
  let count = typed_readonly_ptr::<u32>(&input.field_index(0)).load();
  let scale = typed_readonly_ptr::<f32>(&input.field_index(1)).load();
  let inner = typed_readonly_ptr::<Std430Inner>(&input.field_index(2));
  let items = typed_readonly_ptr::<[Std430Inner]>(&input.field_index(3));

  let output_items = typed_ptr::<[Std430Inner]>(&output.field_index(3));
  typed_ptr::<u32>(&output.field_index(0)).store(items.array_length());
  typed_ptr::<f32>(&output.field_index(1)).store(scale * val(2.));
  typed_ptr::<Std430Inner>(&output.field_index(2)).store(items.index(zero).load());
  items.clone().into_shader_iter().for_each(|(i, item), _| {
    output_items.index(i).a().store(item.a().load() * scale);
    output_items.index(i).v().store(item.v().load());
  });

  vec![
    count.into_f32(),
    scale,
    inner.a().load(),
    inner.v().load().z(),
    items.array_length().into_f32(),
    output_items.array_length().into_f32(),
    items.index(zero + val(1)).a().load(),
    items.index(zero + val(2)).v().load().y(),
  ]
}

fn unsized_struct_host_data() -> (UnsizedHead, Vec<Std430Inner>) {
  let head = UnsizedHead {
    count: 7,
    scale: 3.,
    inner: std430_inner(4., Vec3::new(5., 6., 7.)),
    ..Zeroable::zeroed()
  };
  let items = (0..3)
    .map(|i| {
      let i = i as f32;
      std430_inner(i + 10., Vec3::new(i + 20., i + 30., i + 40.))
    })
    .collect();
  (head, items)
}

fn unsized_struct_buffers() -> [RawBuffer; 2] {
  let (head, items) = unsized_struct_host_data();
  let mut bytes = bytes_of(&head).to_vec();
  bytes.extend_from_slice(cast_slice(&items));
  let empty = vec![0; bytes.len()];
  [
    RawBuffer {
      ty: unsized_struct_ty(),
      space: BufferSpace::Storage,
      bytes,
    },
    RawBuffer {
      ty: unsized_struct_ty(),
      space: BufferSpace::ReadWriteStorage,
      bytes: empty,
    },
  ]
}

/// the struct with trailing runtime sized array, its sized fields, the array length of the
/// trailing array and the writes
#[test]
fn unsized_struct() {
  check_raw_buffers(&unsized_struct_buffers(), unsized_struct_logic);
}

/// the GPU version of [unsized_struct], the runtime sized array starts after the sized fields at
/// the std430 offset
#[pollster::test]
async fn unsized_struct_gpu() {
  let result = gpu_run_raw_buffers(&unsized_struct_buffers(), 1, 8, unsized_struct_logic).await;
  let (head, items) = unsized_struct_host_data();
  let expect = [7., 3., 4., 7., 3., 3., items[1].a, items[2].v.y];
  assert_eq!(result.output[0], expect);

  let (written_head, written_items) = result.buffers[1].split_at(size_of::<UnsizedHead>());
  let written_head: UnsizedHead = pod_read_unaligned(written_head);
  assert_eq!(written_head.count, 3);
  assert_eq!(written_head.scale, head.scale * 2.);
  assert_eq!(
    std430_inner_cpu_leaves(&written_head.inner),
    std430_inner_cpu_leaves(&items[0])
  );
  for (i, written) in read_pods::<Std430Inner>(written_items).iter().enumerate() {
    let v = items[i].v;
    let expect = [items[i].a * head.scale, v.x, v.y, v.z];
    assert_eq!(std430_inner_cpu_leaves(written), expect, "item {i}");
  }
}

/// the workgroup variables of the padded struct, the arrays of struct, the nested array, the host
/// sized array, the atomic array and the matrix, and the workgroup uniform load of the struct,
/// array and the field pointer
#[test]
fn workgroup_shared_vars() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    let padded = builder.define_workgroup_shared_var::<Std140C>();
    padded.b().inner().x().store(f);
    let structs = builder.define_workgroup_shared_var::<[Std140B; 4]>();
    structs.index(u).inner().x().store(f);
    let std430 = builder.define_workgroup_shared_var::<[Std430Inner; 8]>();
    std430.index(u).v().z().store(f);
    let nested = builder.define_workgroup_shared_var::<[[f32; 4]; 4]>();
    nested.index(u).index(u).store(f);
    let host_sized = builder.define_workgroup_shared_var_host_size_array::<Std430Inner>(16);
    host_sized.index(u).a().store(f);
    let atomics = builder.define_workgroup_shared_var::<[DeviceAtomic<u32>; 4]>();
    keep(atomics.index(u).atomic_add(u));
    let matrix = builder.define_workgroup_shared_var::<Mat4<f32>>();
    matrix.index(u).store(f.splat());
    workgroup_barrier();

    keep(workgroup_uniform_load::<Std140C>(padded.clone()).expand().c);
    keep(workgroup_uniform_load::<f32>(padded.b().inner().x()));
    keep(workgroup_uniform_load::<[Std140B; 4]>(structs.clone()));
    keep(workgroup_uniform_load::<Vec3<f32>>(
      std430.index(val(1)).v(),
    ));
    keep(workgroup_uniform_load::<Mat4<f32>>(matrix));
    keep(structs.index(u).load());
    keep(host_sized.index(u).load());
    keep(nested.load());
  });
}

const WORKGROUP_SIZE: u32 = 8;

/// Each invocation writes its slot of the workgroup arrays, and reads the slot of the next
/// invocation after the barrier, the invocation 0 writes the struct that is uniform loaded.
fn workgroup_shared_logic(
  builder: &ShaderComputePipelineBuilder,
  _: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let id = builder.local_invocation_index();
  let padded = builder.define_workgroup_shared_var::<[Std140B; WORKGROUP_SIZE as usize]>();
  let std430 = builder.define_workgroup_shared_var_host_size_array::<Std430Inner>(WORKGROUP_SIZE);
  let uniform = builder.define_workgroup_shared_var::<Std140C>();

  let x = id.into_f32();
  let inner = ENode::<Std140A> { x: x + val(0.5) }.construct();
  let b = ENode::<Std140B> {
    a: id * val(10),
    inner,
  };
  padded.index(id).store(b.construct());
  let v = (x, x + val(1.), x + val(2.)).into();
  std430
    .index(id)
    .store(ENode::<Std430Inner> { a: x * val(2.), v }.construct());
  if_by(id.equals(val(0)), || {
    uniform.store(val(std140_mixed_data().c));
  });
  workgroup_barrier();

  let next = (id + val(1)) % val(WORKGROUP_SIZE);
  let b = padded.index(next).load().expand();
  let inner = std430.index(next).load().expand();
  let c = workgroup_uniform_load::<Std140C>(uniform).expand();
  vec![
    b.a.into_f32(),
    b.inner.expand().x,
    inner.a,
    inner.v.x(),
    inner.v.y(),
    inner.v.z(),
    c.a,
    Std140B::a(c.b).into_f32(),
    c.c,
  ]
}

/// the workgroup variables of the padded struct array and the std430 struct array shared by the
/// invocations, and the workgroup uniform load of the padded struct
#[test]
fn workgroup_shared_struct_array() {
  check_raw_buffers(&[], workgroup_shared_logic);
}

/// the GPU version of [workgroup_shared_struct_array]
#[pollster::test]
async fn workgroup_shared_struct_array_gpu() {
  let result = gpu_run_raw_buffers(&[], WORKGROUP_SIZE, 9, workgroup_shared_logic).await;
  let c = std140_mixed_data().c;
  for (id, output) in result.output.iter().enumerate() {
    let next = ((id as u32 + 1) % WORKGROUP_SIZE) as f32;
    let expect = [
      next * 10.,
      next + 0.5,
      next * 2.,
      next,
      next + 1.,
      next + 2.,
      c.a,
      c.b.a as f32,
      c.c,
    ];
    assert_eq!(output, &expect, "invocation {id}");
  }
}

/// the atomic can not be loaded as a value, `atomic_load` is required
#[test]
#[should_panic(expected = "atomic is not able to direct load")]
fn atomic_direct_load() {
  build_compute(|builder| {
    builder
      .define_workgroup_shared_var::<DeviceAtomic<u32>>()
      .load();
  });
}

/// the private variables of the scalar, matrix, padded struct and array of struct
#[test]
fn private_vars() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);

    let scalar = private_var::<u32>();
    scalar.store(u);
    keep(scalar.load());
    let matrix = private_var::<Mat4<f32>>();
    matrix.index(u).store(f.splat());
    keep(matrix.load());
    let padded = private_var::<Std140C>();
    padded.b().inner().x().store(f);
    keep(padded.load());
    let array = private_var::<[Std430Inner; 4]>();
    array.index(u).v().store(f.splat());
    keep(array.index(u).load());
  });
}

/// Create a binding array in the bind group 0 without any GPU resource container, `storage` is
/// the storage buffer flag for the buffer element.
fn fake_binding_array<T: ShaderNodeSingleType>(
  entry_index: usize,
  count: usize,
  storage: bool,
) -> BindingNode<BindingArray<ShaderBinding<T>>> {
  ShaderInputNode::Binding {
    desc: ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: storage,
      ty: ShaderValueType::BindingArray {
        count,
        ty: T::single_ty(),
      },
      writeable_if_storage: false,
      has_dynamic_offset: false,
    },
    bindgroup_index: 0,
    entry_index,
  }
  .insert_api()
}

/// the binding arrays of textures and samplers indexed by the runtime value
#[test]
fn binding_array_of_textures() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let textures = fake_binding_array::<ShaderTexture2D>(0, 4, false);
    let samplers = fake_binding_array::<ShaderSampler>(1, 4, false);
    let depth = fake_binding_array::<ShaderDepthTexture2D>(2, 2, false);

    keep(
      textures
        .index(u)
        .sample_zero_level(samplers.index(u), f.splat()),
    );
    keep(textures.index(val(1)).texture_dimension_2d(None));
    keep(depth.index(u).load_texel(u.splat(), val(0)));
  });
}

/// Create a binding array of buffers in the address space without any GPU resource container.
fn fake_buffer_binding_array<T: ShaderNodeSingleType>(
  entry_index: usize,
  count: usize,
  space: BufferSpace,
) -> BindingNode<BindingArray<ShaderBinding<T>>> {
  let ty = ShaderValueType::BindingArray {
    count,
    ty: T::single_ty(),
  };
  ShaderInputNode::Binding {
    desc: buffer_binding_desc(ty, space),
    bindgroup_index: 0,
    entry_index,
  }
  .insert_api()
}

/// the binding arrays of readonly storage, read_write storage and uniform buffers of the padded
/// structs, the fields are accessed through the indexed buffer
#[test]
fn binding_array_of_buffers() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;

    let readonly = fake_buffer_binding_array::<Std430Inner>(0, 2, BufferSpace::Storage);
    let buffer = readonly.index_readonly_buffer(u);
    keep(buffer.a().load());
    keep(buffer.v().load());

    let read_write = fake_buffer_binding_array::<Std430Inner>(1, 2, BufferSpace::ReadWriteStorage);
    let buffer = read_write.index_buffer(u);
    buffer.a().store(buffer.v().load().z());

    let uniform = fake_buffer_binding_array::<Std140C>(2, 2, BufferSpace::Uniform);
    let buffer = uniform.index_readonly_buffer(val(1));
    keep(buffer.b().inner().x().load());
    keep(buffer.c().load());
  });
}

/// the element of the buffer binding array must be a struct
#[test]
#[should_panic(expected = "the element of the buffer binding array must be a struct")]
fn binding_array_of_non_struct_buffers() {
  build_compute(|_| {
    fake_buffer_binding_array::<Vec4<f32>>(0, 2, BufferSpace::Storage);
  });
}

type StorageBufferArray = BindingResourceArray<StorageBufferReadonlyDataView<Std430Inner>>;

/// Read the fields of each buffer of the binding array, and write them to the output.
fn read_storage_buffer_array(
  builder: &mut ShaderComputePipelineBuilder,
  array: &StorageBufferArray,
  output: &StorageBufferDataView<[f32]>,
) {
  let array = builder.bind_by(array);
  let output = builder.bind_by(output);
  for i in 0..2 {
    let buffer = array.index_readonly_buffer(val(i));
    output
      .index(val(i))
      .store(buffer.a().load() + buffer.v().load().z());
  }
}

/// the binding array container of readonly storage buffers declares the readonly storage binding
/// in the shader and the layout, and the indexed buffers are read on the GPU if the device supports
/// the buffer binding array
#[pollster::test]
async fn binding_array_of_storage_buffers_gpu() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let items = [
    std430_inner(1., Vec3::new(2., 3., 4.)),
    std430_inner(5., Vec3::new(6., 7., 8.)),
  ];
  let buffers = items
    .iter()
    .map(|item| create_gpu_readonly_storage(item, &gpu, "binding array element"))
    .collect();
  let array = BindingResourceArray::new(Arc::new(buffers), 2, &gpu.device);
  let output = create_gpu_read_write_storage::<[f32]>(
    ZeroedArrayByArrayLength(items.len()),
    &gpu,
    "binding array output",
  );

  let desc = array.binding_desc();
  assert!(desc.should_as_storage_buffer_if_is_buffer_like && !desc.writeable_if_storage);
  let entry = map_shader_value_ty_to_binding_layout_type(&desc, 0, ShaderStages::COMPUTE);
  assert_eq!(entry.count, std::num::NonZeroU32::new(2));
  assert!(matches!(
    entry.ty,
    BindingType::Buffer {
      ty: BufferBindingType::Storage { read_only: true },
      ..
    }
  ));

  let mut builder = ShaderComputePipelineBuilder::new(
    &|stage| {
      Box::new(rendiation_shader_backend_naga::ShaderAPINagaImpl::new(
        stage,
      ))
    },
    ShaderRuntimeChecks::default(),
  );
  read_storage_buffer_array(&mut builder, &array, &output);
  let result = builder.build().expect("failed to build shader");
  validate(
    &result
      .shader
      .1
      .downcast::<rendiation_shader_backend_naga::NagaModuleBuildResult>()
      .unwrap()
      .module,
  );

  let required = Features::BUFFER_BINDING_ARRAY | Features::STORAGE_RESOURCE_BINDING_ARRAY;
  if !gpu.info().supported_features.contains(required) {
    println!("the GPU execution is skipped, the buffer binding array is not supported");
    return;
  }

  let hasher = shader_hasher_from_marker_ty!(BufferBindingArray);
  let pipeline = gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(1);
      read_storage_buffer_array(&mut builder, &array, &output);
      builder
    });

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(&array)
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);
  });
  let result = encoder.read_buffer(&gpu.device, &output);
  gpu.submit_encoder(encoder);
  let result = result.await.unwrap();
  let result = <[f32]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
  assert_eq!(result, [5., 13.]);
}

/// Define and call a user function without parameter, its body captures the nodes created in the
/// entry function.
fn call_fn(name: &str, body: impl FnOnce() -> Node<f32>) -> Node<f32> {
  get_shader_fn::<f32>(name.to_owned())
    .or_define(|cx| cx.do_return(body()))
    .prepare_parameters()
    .call()
}

/// the storage binding created in the entry function is read and written in a user function
#[test]
fn binding_in_function() {
  check_compute(|_| {
    let storage = fake_storage_buffer::<[f32]>(0);
    keep(call_fn("binding_in_function", || {
      storage.index(val(1)).store(val(1.));
      storage.index(val(0)).load()
    }));
  });
}

/// the uniform binding of the padded struct is accessed through the field pointer in a user
/// function
#[test]
fn padded_uniform_in_function() {
  check_compute(|_| {
    let uniform = typed_readonly_ptr::<Std140C>(&fake_buffer::<Std140C>(0, BufferSpace::Uniform));
    keep(call_fn("padded_uniform_in_function", || {
      uniform.b().inner().x().load() + uniform.c().load()
    }));
  });
}

/// the workgroup variable created in the entry function is accessed in a user function
#[test]
fn workgroup_var_in_function() {
  check_compute(|builder| {
    let shared = builder.define_workgroup_shared_var::<[f32; 4]>();
    keep(call_fn("workgroup_var_in_function", || {
      shared.index(val(1)).store(val(1.));
      shared.index(val(0)).load()
    }));
  });
}

/// the atomic and the workgroup uniform load are called directly on the workgroup variables
/// created in the entry function, in a user function
#[test]
fn workgroup_var_statement_in_function() {
  check_compute(|builder| {
    let atomic = builder.define_workgroup_shared_var::<DeviceAtomic<u32>>();
    let shared = builder.define_workgroup_shared_var::<f32>();
    keep(call_fn("workgroup_var_statement_in_function", || {
      atomic.atomic_add(val(1)).into_f32() + workgroup_uniform_load::<f32>(shared)
    }));
  });
}

/// the private variable created in the entry function is accessed in a user function
#[test]
fn private_var_in_function() {
  check_compute(|_| {
    let private = private_var::<f32>();
    keep(call_fn("private_var_in_function", || {
      private.store(private.load() + val(1.));
      private.load()
    }));
  });
}

/// the constant of the padded struct created in the entry function is used in a user function
#[test]
fn constant_in_function() {
  check_compute(|_| {
    let constant = global_const_val(std140_mixed_data().c);
    let inlined = val(std140_b(1, 2.));
    keep(call_fn("constant_in_function", || {
      Std140C::c(constant) + Std140A::x(Std140B::inner(inlined))
    }));
  });
}

const FN_BUFFER_LEN: usize = 6;

/// The raw buffers are the readonly storage of [FN_BUFFER_LEN] f32, the read_write storage of the
/// same length and the uniform [Std140C]. They are declared in the entry and accessed in a user
/// function, which only takes the invocation index as parameter.
fn bindings_in_function_logic(
  builder: &ShaderComputePipelineBuilder,
  buffers: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let input = typed_readonly_ptr::<[f32]>(&buffers[0]);
  let output = typed_ptr::<[f32]>(&buffers[1]);
  let uniform = typed_readonly_ptr::<Std140C>(&buffers[2]);
  let id = builder.local_invocation_index();
  let returned = get_shader_fn::<f32>("bindings_in_function".to_owned())
    .or_define(|cx| {
      let id = cx.push_fn_parameter_by(id);
      let mirrored = input.index(input.array_length() - val(1) - id).load();
      let v = input.index(id).load() * uniform.b().inner().x().load() + mirrored;
      output.index(id).store(v);
      cx.do_return(v + uniform.c().load());
    })
    .prepare_parameters()
    .push(id)
    .call();
  vec![returned]
}

fn bindings_in_function_input() -> Vec<f32> {
  (0..FN_BUFFER_LEN).map(|i| i as f32 * 1.5 + 1.).collect()
}

fn bindings_in_function_buffers() -> [RawBuffer; 3] {
  [
    RawBuffer {
      ty: <[f32]>::ty(),
      space: BufferSpace::Storage,
      bytes: cast_slice(&bindings_in_function_input()).to_vec(),
    },
    RawBuffer {
      ty: <[f32]>::ty(),
      space: BufferSpace::ReadWriteStorage,
      bytes: vec![0; size_of::<f32>() * FN_BUFFER_LEN],
    },
    RawBuffer {
      ty: Std140C::ty(),
      space: BufferSpace::Uniform,
      bytes: bytes_of(&std140_mixed_data().c).to_vec(),
    },
  ]
}

/// the storage and the padded uniform bindings declared in the entry are read and written in a
/// user function
#[test]
fn bindings_in_function_body() {
  check_raw_buffers(&bindings_in_function_buffers(), bindings_in_function_logic);
}

/// the GPU version of [bindings_in_function_body], one invocation for each item
#[pollster::test]
async fn bindings_in_function_body_gpu() {
  let result = gpu_run_raw_buffers(
    &bindings_in_function_buffers(),
    FN_BUFFER_LEN as u32,
    1,
    bindings_in_function_logic,
  )
  .await;

  let input = bindings_in_function_input();
  let c = std140_mixed_data().c;
  let expect: Vec<_> = (0..FN_BUFFER_LEN)
    .map(|id| input[id] * c.b.inner.x + input[FN_BUFFER_LEN - 1 - id])
    .collect();
  assert_eq!(read_pods::<f32>(&result.buffers[1]), expect);
  for (id, output) in result.output.iter().enumerate() {
    assert_eq!(output, &[expect[id] + c.c], "invocation {id}");
  }
}
