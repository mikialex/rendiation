use rendiation_webgpu_virtual_typed_combine_buffer::CombinedStorageBufferAllocator;

use crate::*;

mod rgba;

/// a small width to cover the multi row cases
const TEST_WIDTH: u32 = 8;

fn allocators(gpu: &GPU) -> [(&'static str, TextureAsStorageAllocator); 2] {
  [
    (
      "direct",
      TextureAsStorageAllocator::new(gpu).with_max_width(TEST_WIDTH),
    ),
    (
      "host",
      TextureAsStorageAllocator::new_with_host_backup(gpu).with_max_width(TEST_WIDTH),
    ),
  ]
}

/// run a compute shader that writes `output[i] = read(input, i)` for each output element
async fn read_by_shader<S>(
  gpu: &GPU,
  source: &S,
  output_len: usize,
  hash_key: &str,
  read: impl Fn(&S::ShaderBindResult, Node<u32>) -> Node<u32>,
) -> Vec<u32>
where
  S: AbstractShaderBindingSource + AbstractBindingSource,
{
  let output =
    create_gpu_read_write_storage::<[u32]>(ZeroedArrayByArrayLength(output_len), gpu, "output");

  let hasher = shader_hasher_from_marker_ty!(ReadByShader).with_hash(hash_key);
  let pipeline = gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(64);
      let input = builder.bind_by(source);
      let out = builder.bind_by(&output);
      let id = builder.global_invocation_id().x();
      if_by(id.less_than(out.array_length()), || {
        out.index(id).store(read(&input, id));
      });
      builder
    });

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(source)
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups((output_len as u32).div_ceil(64), 1, 1);
  });
  let result = encoder.read_storage_array(&gpu.device, &output);
  gpu.submit_encoder(encoder);
  result.await.unwrap()
}

/// return the array length read in shader, and the content
async fn read_u32_array(
  gpu: &GPU,
  buffer: &AbstractReadonlyStorageBuffer<[u32]>,
  hash_key: &str,
) -> (u32, Vec<u32>) {
  let len = buffer.item_count() as usize;
  let r = read_by_shader(gpu, buffer, len + 1, hash_key, |input, id| {
    let is_len = id.equals(val(0));
    let index = is_len.select(val(0), id - val(1));
    is_len.select(input.array_length(), input.index(index).load())
  })
  .await;
  (r[0], r[1..].to_vec())
}

fn write_u32s(
  buffer: &AbstractReadonlyStorageBuffer<[u32]>,
  data: &[u32],
  offset: usize,
  gpu: &GPU,
) {
  buffer.write(cast_slice(data), offset as u64 * 4, &gpu.queue);
}

fn resize(buffer: &mut AbstractReadonlyStorageBuffer<[u32]>, len: usize, gpu: &GPU) -> bool {
  let mut encoder = gpu.create_encoder();
  let r = buffer.resize_gpu(&mut encoder, &gpu.device, len as u64 * 4, None);
  gpu.submit_encoder(encoder);
  r
}

fn relocation(src: usize, dst: usize, count: usize) -> BufferRelocate {
  BufferRelocate {
    self_offset: src as u64 * 4,
    target_offset: dst as u64 * 4,
    count: count as u64 * 4,
  }
}

/// apply the relocations to the model, the sources refer to the content before relocation
fn relocate_model(model: &mut [u32], relocations: &[(usize, usize, usize)]) {
  let snapshot = model.to_vec();
  for &(src, dst, count) in relocations {
    model[dst..dst + count].copy_from_slice(&snapshot[src..src + count]);
  }
}

#[pollster::test]
async fn fragmented_write() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    for len in [0, 1, 6, 7, 8, 9, 15, 23, 40] {
      let buffer = alloc.allocate_readonly::<[u32]>(len as u64 * 4, &gpu.device, "test");
      let mut model = vec![0; len];

      // writes with different size that cross the row boundary
      let mut start = 0;
      let mut size = 1;
      while start < len {
        let end = (start + size).min(len);
        let data: Vec<_> = (start..end).map(|i| i as u32 + 100).collect();
        write_u32s(&buffer, &data, start, &gpu);
        model[start..end].copy_from_slice(&data);
        start = end + 1;
        size = size % 11 + 3;
      }
      if len > 0 {
        write_u32s(&buffer, &[7], len / 2, &gpu);
        model[len / 2] = 7;
      }

      let (array_len, data) = read_u32_array(&gpu, &buffer, "u32").await;
      assert_eq!(array_len, len as u32, "{name} len {len}");
      assert_eq!(data, model, "{name} len {len}");
    }
  }
}

fn pick_component(v: Node<Vec4<u32>>, c: Node<u32>) -> Node<u32> {
  c.equals(val(0)).select(
    v.x(),
    c.equals(val(1))
      .select(v.y(), c.equals(val(2)).select(v.z(), v.w())),
  )
}

#[pollster::test]
async fn typed_access() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let data: Vec<_> = (0..5_u32)
      .map(|i| Vec4::new(i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 3))
      .collect();
    let buffer = alloc.allocate_readonly_init(data.as_slice(), &gpu, "vec4 array");
    let r = read_by_shader(
      &gpu,
      &buffer,
      1 + data.len() * 4,
      "vec4 array",
      |input, id| {
        let is_len = id.equals(val(0));
        let index = is_len.select(val(0), id - val(1));
        let item = pick_component(input.index(index / val(4)).load(), index % val(4));
        is_len.select(input.array_length(), item)
      },
    )
    .await;
    let expect: Vec<_> = std::iter::once(5).chain(0..20).collect();
    assert_eq!(r, expect, "{name}");

    let sized = alloc.allocate_readonly_init(&Vec4::new(1_u32, 2, 3, 4), &gpu, "sized");
    let r = read_by_shader(&gpu, &sized, 4, "sized vec4", |input, id| {
      pick_component(input.load(), id)
    })
    .await;
    assert_eq!(r, vec![1, 2, 3, 4], "{name}");
  }
}

/// a u32 field followed by a runtime sized vec4 array, the array starts at the 4th u32 in std430
fn unsized_struct_ty() -> MaybeUnsizedValueType {
  let ty: &'static _ = Box::leak(Box::new(ShaderUnSizedStructMetaInfo {
    name: "TextureAsBufferTestUnsizedStruct".into(),
    sized_fields: vec![ShaderStructFieldMetaInfo {
      name: "count".into(),
      ty: u32::sized_ty(),
      ty_deco: None,
    }],
    last_dynamic_array_field: ("data".into(), Box::new(Vec4::<u32>::sized_ty())),
  }));
  MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedStruct(ty))
}

/// return the array length of the last field, the sized field, and the array content
async fn read_unsized_struct(
  gpu: &GPU,
  buffer: &BoxedAbstractBuffer,
  array_len: usize,
  hash_key: &str,
) -> (u32, u32, Vec<u32>) {
  let r = read_by_shader(gpu, buffer, 2 + array_len * 4, hash_key, |ptr, id| {
    let array = ptr.field_index(1);
    let count: Node<u32> = unsafe { ptr.field_index(0).load().into_node() };
    let index = id.less_than(val(2)).select(val(0), id - val(2));
    let item = unsafe { array.field_array_index(index / val(4)).load().into_node() };
    let item = pick_component(item, index % val(4));
    let rest = id.equals(val(1)).select(count, item);
    id.equals(val(0)).select(array.array_length(), rest)
  })
  .await;
  (r[0], r[1], r[2..].to_vec())
}

#[pollster::test]
async fn unsized_struct_array_length() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let combine =
      CombinedStorageBufferAllocator::new(&gpu, "combine", false, true, Box::new(alloc.clone()));
    let cases: [(&str, &dyn AbstractStorageAllocator); 2] =
      [("texture", &alloc), ("combine", &combine)];

    for (case, allocator) in cases {
      let content: Vec<_> = [7, 0, 0, 0].into_iter().chain(100..112).collect();
      let mut buffer =
        allocator.allocate_dyn_ty(16 + 3 * 16, &gpu.device, unsized_struct_ty(), true, "test");
      buffer.write(cast_slice(&content), 0, &gpu.queue);

      let key = format!("{case} unsized struct");
      let mut expect = content[4..].to_vec();
      let r = read_unsized_struct(&gpu, &buffer, 3, &key).await;
      assert_eq!(r, (3, 7, expect.clone()), "{name} {case}");

      let mut encoder = gpu.create_encoder();
      assert!(buffer.resize_gpu(&mut encoder, &gpu.device, 16 + 5 * 16, None));
      gpu.submit_encoder(encoder);
      expect.resize(20, 0);
      let r = read_unsized_struct(&gpu, &buffer, 5, &key).await;
      assert_eq!(r, (5, 7, expect), "{name} {case}");
    }
  }
}

#[pollster::test]
async fn resize_and_relocate() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let mut buffer = alloc.allocate_readonly::<[u32]>(5 * 4, &gpu.device, "test");
    let mut model: Vec<_> = (1..=5).collect();
    write_u32s(&buffer, &model, 0, &gpu);

    let check = async |buffer: &AbstractReadonlyStorageBuffer<[u32]>, model: &Vec<u32>, step| {
      let (array_len, data) = read_u32_array(&gpu, buffer, "u32").await;
      assert_eq!(array_len, model.len() as u32, "{name} {step}");
      assert_eq!(&data, model, "{name} {step}");
    };

    // within the same row
    assert!(resize(&mut buffer, 6, &gpu));
    model.resize(6, 0);
    check(&buffer, &model, "grow in row").await;

    assert!(resize(&mut buffer, 20, &gpu));
    model.resize(20, 0);
    write_u32s(&buffer, &[99], 19, &gpu);
    model[19] = 99;
    check(&buffer, &model, "grow to rows").await;

    assert!(resize(&mut buffer, 3, &gpu));
    model.truncate(3);
    check(&buffer, &model, "shrink").await;

    // the truncated part must be zeroed when grow back
    assert!(resize(&mut buffer, 21, &gpu));
    model.resize(21, 0);
    check(&buffer, &model, "grow back").await;

    let data: Vec<_> = (0..21).map(|i| i + 200).collect();
    write_u32s(&buffer, &data, 0, &gpu);
    model.copy_from_slice(&data);

    // overlapped relocations that cross the row boundary
    let relocations = [(0, 3, 6), (10, 12, 5), (19, 1, 2)];
    let mut encoder = gpu.create_encoder();
    let mut iter = relocations.iter().map(|&(s, d, c)| relocation(s, d, c));
    buffer.batch_self_relocate(&mut iter, &mut encoder, &gpu.device);
    gpu.submit_encoder(encoder);
    relocate_model(&mut model, &relocations);
    check(&buffer, &model, "relocate").await;

    let mut encoder = gpu.create_encoder();
    buffer.batch_self_relocate(&mut std::iter::empty(), &mut encoder, &gpu.device);
    gpu.submit_encoder(encoder);
    check(&buffer, &model, "empty relocate").await;

    // the relocation sources refer to the content before resize
    let relocations = [(16, 2, 5), (0, 9, 3)];
    let mut encoder = gpu.create_encoder();
    let mut iter = relocations.iter().map(|&(s, d, c)| relocation(s, d, c));
    let r = buffer.resize_gpu(&mut encoder, &gpu.device, 12 * 4, Some(&mut iter));
    gpu.submit_encoder(encoder);
    assert!(r);
    let snapshot = model.clone();
    model.truncate(12);
    for (src, dst, count) in relocations {
      model[dst..dst + count].copy_from_slice(&snapshot[src..src + count]);
    }
    check(&buffer, &model, "resize with relocations").await;

    let limit = device_texel_limit(&gpu.device, Some(TEST_WIDTH));
    assert!(!resize(&mut buffer, limit.texel_count() as usize, &gpu));
    assert_eq!(buffer.item_count(), 12, "{name}");
    assert!(resize(&mut buffer, limit.texel_count() as usize - 1, &gpu));
  }
}

#[pollster::test]
async fn grow_with_far_write() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let mut buffer = alloc.allocate_readonly::<[u32]>(10 * 4, &gpu.device, "test");
    let mut model: Vec<_> = (0..10).map(|i| i + 1).collect();
    write_u32s(&buffer, &model, 0, &gpu);
    assert_eq!(
      read_u32_array(&gpu, &buffer, "u32").await.1,
      model,
      "{name}"
    );

    // the dirty ranges are too far to merge, so the old content must be kept by the texture copy
    let len = 3000;
    assert!(resize(&mut buffer, len, &gpu));
    model.resize(len, 0);
    write_u32s(&buffer, &[42], len - 1, &gpu);
    model[len - 1] = 42;

    let (array_len, data) = read_u32_array(&gpu, &buffer, "u32").await;
    assert_eq!(array_len, len as u32, "{name}");
    assert_eq!(data, model, "{name}");
  }
}

#[pollster::test]
async fn copy_between_buffers() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let src = alloc.allocate_readonly::<[u32]>(20 * 4, &gpu.device, "src");
    let dst = alloc.allocate_readonly::<[u32]>(30 * 4, &gpu.device, "dst");
    let src_data: Vec<_> = (0..20).map(|i| i + 1).collect();
    write_u32s(&src, &src_data, 0, &gpu);
    let mut model: Vec<_> = (0..30).map(|i| i + 1000).collect();
    write_u32s(&dst, &model, 0, &gpu);

    // the in row offset of src and dst is different
    let mut encoder = gpu.create_encoder();
    src.copy_buffer_to_buffer(&*dst, 2 * 4, 9 * 4, 12 * 4, &mut encoder);
    gpu.submit_encoder(encoder);
    model[9..21].copy_from_slice(&src_data[2..14]);

    let (_, data) = read_u32_array(&gpu, &dst, "u32").await;
    assert_eq!(data, model, "{name}");
  }
}

#[pollster::test]
async fn combined_buffer_on_texture() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  for (name, alloc) in allocators(&gpu) {
    let combine =
      CombinedStorageBufferAllocator::new(&gpu, "combine", false, true, Box::new(alloc));

    let a = combine.allocate_readonly::<[u32]>(13 * 4, &gpu.device, "a");
    let b = combine.allocate_readonly::<[u32]>(30 * 4, &gpu.device, "b");
    let a_data: Vec<_> = (0..13).map(|i| i + 1).collect();
    let mut b_data: Vec<_> = (0..30).map(|i| i + 100).collect();
    write_u32s(&a, &a_data, 0, &gpu);
    write_u32s(&b, &b_data, 0, &gpu);

    assert_eq!(
      read_u32_array(&gpu, &a, "combine u32").await,
      (13, a_data.clone()),
      "{name}"
    );
    assert_eq!(
      read_u32_array(&gpu, &b, "combine u32").await,
      (30, b_data.clone()),
      "{name}"
    );

    // the new allocation rebuilds the combined buffer, the old content is copied into the new one
    let c = combine.allocate_readonly::<[u32]>(4 * 4, &gpu.device, "c");
    write_u32s(&c, &[5, 6, 7, 8], 0, &gpu);
    write_u32s(&b, &[1, 2, 3], 7, &gpu);
    b_data[7..10].copy_from_slice(&[1, 2, 3]);

    assert_eq!(
      read_u32_array(&gpu, &a, "combine u32").await,
      (13, a_data),
      "{name}"
    );
    assert_eq!(
      read_u32_array(&gpu, &b, "combine u32").await,
      (30, b_data),
      "{name}"
    );
    assert_eq!(
      read_u32_array(&gpu, &c, "combine u32").await,
      (4, vec![5, 6, 7, 8]),
      "{name}"
    );
  }
}
