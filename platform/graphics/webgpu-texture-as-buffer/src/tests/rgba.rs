use super::*;

/// with the test width, a row has 32 u32
fn create(gpu: &GPU, byte_size: u64, ty: MaybeUnsizedValueType) -> BoxedAbstractBuffer {
  let limit = device_texel_limit(&gpu.device, Some(TEST_WIDTH));
  Box::new(RgbaTextureAsReadonlyStorageBuffer::new(
    gpu, byte_size, ty, limit, "test",
  ))
}

fn create_u32_array(gpu: &GPU, len: usize) -> BoxedAbstractBuffer {
  create(gpu, len as u64 * 4, <[u32]>::maybe_unsized_ty())
}

fn write(buffer: &BoxedAbstractBuffer, data: &[u32], offset: usize, gpu: &GPU) {
  buffer.write(cast_slice(data), offset as u64 * 4, &gpu.queue);
}

fn resize(buffer: &mut BoxedAbstractBuffer, len: usize, gpu: &GPU) -> bool {
  let mut encoder = gpu.create_encoder();
  let r = buffer.resize_gpu(&mut encoder, &gpu.device, len as u64 * 4, None);
  gpu.submit_encoder(encoder);
  r
}

/// return the array length read in shader, and the content
async fn read_u32_heap(gpu: &GPU, buffer: &BoxedAbstractBuffer) -> (u32, Vec<u32>) {
  let len = buffer.byte_size() as usize / 4;
  let r = read_by_shader(gpu, buffer, len + 1, "rgba u32", |ptr, id| {
    let input = <[u32]>::create_readonly_view_from_raw_ptr(ptr.clone());
    let is_len = id.equals(val(0));
    let index = is_len.select(val(0), id - val(1));
    is_len.select(input.array_length(), input.index(index).load())
  })
  .await;
  (r[0], r[1..].to_vec())
}

#[pollster::test]
async fn rgba_whole_write() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  // around the texel boundary, and the extent that is exactly filled by the data and the length
  for len in [0, 1, 3, 4, 5, 27, 28, 29, 32, 60, 61, 100] {
    let buffer = create_u32_array(&gpu, len);
    let data: Vec<_> = (0..len as u32).map(|i| i + 100).collect();
    write(&buffer, &data, 0, &gpu);
    let r = read_u32_heap(&gpu, &buffer).await;
    assert_eq!(r, (len as u32, data), "len {len}");
  }
}

#[pollster::test]
async fn rgba_chunked_write() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let len = 50;
  let buffer = create_u32_array(&gpu, len);
  let mut model: Vec<_> = (0..len as u32).map(|i| i * 3 + 1).collect();

  // the last chunk ends at the byte size, which is not 16 bytes aligned
  for chunk in [0..16, 16..32, 32..50] {
    write(&buffer, &model[chunk.clone()], chunk.start, &gpu);
  }
  write(&buffer, &[7; 4], 8, &gpu);
  model[8..12].fill(7);

  assert_eq!(read_u32_heap(&gpu, &buffer).await, (50, model));
}

#[pollster::test]
async fn rgba_typed_access() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let data: Vec<_> = (0..5_u32)
    .map(|i| Vec4::new(i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 3))
    .collect();
  let buffer = create(&gpu, 5 * 16, <[Vec4<u32>]>::maybe_unsized_ty());
  buffer.write(cast_slice(&data), 0, &gpu.queue);
  let r = read_by_shader(&gpu, &buffer, 1 + 20, "rgba vec4 array", |ptr, id| {
    let input = <[Vec4<u32>]>::create_readonly_view_from_raw_ptr(ptr.clone());
    let is_len = id.equals(val(0));
    let index = is_len.select(val(0), id - val(1));
    let item = pick_component(input.index(index / val(4)).load(), index % val(4));
    is_len.select(input.array_length(), item)
  })
  .await;
  let expect: Vec<_> = std::iter::once(5).chain(0..20).collect();
  assert_eq!(r, expect);

  let sized = create(&gpu, 16, <Vec4<u32>>::maybe_unsized_ty());
  sized.write(cast_slice(&[1_u32, 2, 3, 4]), 0, &gpu.queue);
  let r = read_by_shader(&gpu, &sized, 4, "rgba sized vec4", |ptr, id| {
    let input = <Vec4<u32>>::create_readonly_view_from_raw_ptr(ptr.clone());
    pick_component(input.load(), id)
  })
  .await;
  assert_eq!(r, vec![1, 2, 3, 4]);

  // the sized field is in the first texel, and the array starts from the second texel
  let content: Vec<_> = [7, 0, 0, 0].into_iter().chain(100..112).collect();
  let unsized_struct = create(&gpu, 16 + 3 * 16, unsized_struct_ty());
  unsized_struct.write(cast_slice(&content), 0, &gpu.queue);
  let r = read_unsized_struct(&gpu, &unsized_struct, 3, "rgba unsized struct").await;
  assert_eq!(r, (3, 7, content[4..].to_vec()));
}

#[pollster::test]
async fn rgba_resize_and_relocate() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut buffer = create_u32_array(&gpu, 5);
  let mut model: Vec<_> = (1..=5).collect();
  write(&buffer, &model, 0, &gpu);

  let check = async |buffer: &BoxedAbstractBuffer, model: &Vec<u32>, step| {
    let (array_len, data) = read_u32_heap(&gpu, buffer).await;
    assert_eq!(array_len, model.len() as u32, "{step}");
    assert_eq!(&data, model, "{step}");
  };

  // the components beyond the unaligned old size are zero
  assert!(resize(&mut buffer, 9, &gpu));
  model.resize(9, 0);
  check(&buffer, &model, "grow from unaligned size").await;

  // the reallocation must keep the last partial texel of the unaligned old size
  write(&buffer, &[5, 6, 7, 8, 9], 4, &gpu);
  model[4..9].copy_from_slice(&[5, 6, 7, 8, 9]);
  assert!(resize(&mut buffer, 70, &gpu));
  model.resize(70, 0);
  write(&buffer, &[98, 99], 68, &gpu);
  model[68..70].copy_from_slice(&[98, 99]);
  check(&buffer, &model, "grow to rows").await;

  assert!(resize(&mut buffer, 8, &gpu));
  model.truncate(8);
  check(&buffer, &model, "shrink").await;

  // the truncated part must be zeroed when grow back
  assert!(resize(&mut buffer, 20, &gpu));
  model.resize(20, 0);
  check(&buffer, &model, "grow back").await;

  let data: Vec<_> = (0..20).map(|i| i + 200).collect();
  write(&buffer, &data, 0, &gpu);
  model.copy_from_slice(&data);

  // overlapped relocations, the units are u32
  let relocations = [(0, 4, 8), (12, 16, 4)];
  let mut encoder = gpu.create_encoder();
  let mut iter = relocations.iter().map(|&(s, d, c)| relocation(s, d, c));
  buffer.batch_self_relocate(&mut iter, &mut encoder, &gpu.device);
  gpu.submit_encoder(encoder);
  relocate_model(&mut model, &relocations);
  check(&buffer, &model, "relocate").await;

  // the relocation sources refer to the content before resize
  let relocations = [(16, 0, 4), (0, 20, 4)];
  let mut encoder = gpu.create_encoder();
  let mut iter = relocations.iter().map(|&(s, d, c)| relocation(s, d, c));
  let r = buffer.resize_gpu(&mut encoder, &gpu.device, 24 * 4, Some(&mut iter));
  gpu.submit_encoder(encoder);
  assert!(r);
  let snapshot = model.clone();
  model.resize(24, 0);
  for (src, dst, count) in relocations {
    model[dst..dst + count].copy_from_slice(&snapshot[src..src + count]);
  }
  check(&buffer, &model, "resize with relocations").await;

  // the capacity is 4 u32 per texel
  let limit = device_texel_limit(&gpu.device, Some(TEST_WIDTH));
  let max_len = (limit.texel_count() as usize - 1) * 4;
  assert!(!resize(&mut buffer, max_len + 1, &gpu));
  assert_eq!(buffer.byte_size(), 24 * 4);
  assert!(resize(&mut buffer, max_len, &gpu));
}

#[pollster::test]
async fn rgba_copy_between_buffers() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let src = create_u32_array(&gpu, 20);
  let dst = create_u32_array(&gpu, 30);
  let src_data: Vec<_> = (0..20).map(|i| i + 1).collect();
  write(&src, &src_data, 0, &gpu);
  let mut model: Vec<_> = (0..30).map(|i| i + 1000).collect();
  write(&dst, &model, 0, &gpu);

  // the in row offset of src and dst is different
  let mut encoder = gpu.create_encoder();
  src.copy_buffer_to_buffer(&*dst, 4 * 4, 8 * 4, 12 * 4, &mut encoder);
  gpu.submit_encoder(encoder);
  model[8..20].copy_from_slice(&src_data[4..16]);
  assert_eq!(read_u32_heap(&gpu, &dst).await.1, model);

  // the unaligned count is allowed when the copy reaches the end of both side
  let src = create_u32_array(&gpu, 10);
  let dst = create_u32_array(&gpu, 14);
  let src_data: Vec<_> = (0..10).map(|i| i + 1).collect();
  write(&src, &src_data, 0, &gpu);
  let mut model: Vec<_> = (0..14).map(|i| i + 1000).collect();
  write(&dst, &model, 0, &gpu);

  let mut encoder = gpu.create_encoder();
  src.copy_buffer_to_buffer(&*dst, 8 * 4, 12 * 4, 2 * 4, &mut encoder);
  gpu.submit_encoder(encoder);
  model[12..14].copy_from_slice(&src_data[8..10]);
  assert_eq!(read_u32_heap(&gpu, &dst).await.1, model);
}

#[pollster::test]
#[should_panic(expected = "write offset must be 16 bytes aligned")]
async fn rgba_unaligned_write_offset() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let buffer = create_u32_array(&gpu, 8);
  write(&buffer, &[1], 1, &gpu);
}

#[pollster::test]
#[should_panic(expected = "write end, unless it equals the byte size, must be 16 bytes aligned")]
async fn rgba_unaligned_write_end() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let buffer = create_u32_array(&gpu, 8);
  write(&buffer, &[1], 0, &gpu);
}

#[pollster::test]
#[should_panic(expected = "copy source offset must be 16 bytes aligned")]
async fn rgba_unaligned_copy_offset() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let src = create_u32_array(&gpu, 8);
  let dst = create_u32_array(&gpu, 8);
  let mut encoder = gpu.create_encoder();
  src.copy_buffer_to_buffer(&*dst, 4, 0, 16, &mut encoder);
}

#[pollster::test]
#[should_panic(expected = "copy byte count, unless the copy reaches the end of both side,")]
async fn rgba_unaligned_copy_count() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let src = create_u32_array(&gpu, 8);
  let dst = create_u32_array(&gpu, 8);
  let mut encoder = gpu.create_encoder();
  src.copy_buffer_to_buffer(&*dst, 0, 0, 4, &mut encoder);
}

#[pollster::test]
#[should_panic(expected = "relocation source offset must be 16 bytes aligned")]
async fn rgba_unaligned_relocation() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let buffer = create_u32_array(&gpu, 16);
  let mut encoder = gpu.create_encoder();
  buffer.batch_self_relocate(
    &mut std::iter::once(relocation(1, 8, 4)),
    &mut encoder,
    &gpu.device,
  );
}

#[pollster::test]
#[should_panic(expected = "shrink target size must be 16 bytes aligned")]
async fn rgba_unaligned_shrink() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut buffer = create_u32_array(&gpu, 8);
  resize(&mut buffer, 5, &gpu);
}
