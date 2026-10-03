use crate::*;

/// The texture as readonly storage buffer that uses the Rgba32Uint texture, without host backup.
///
/// Each texel stores 16 bytes, so the capacity is 4 times of the R32Uint containers under the same
/// texture size limit. As the texture can only be written and copied in whole texels, the partial
/// update is restricted, the following constraints are asserted:
///
/// - write: the offset is 16 bytes aligned, the end is 16 bytes aligned or equals to the byte size.
/// - copy: the offsets are 16 bytes aligned, the count is 16 bytes aligned, or the copy range ends
///   at the byte size of both side.
/// - relocation: the offsets and the count are 16 bytes aligned.
/// - resize: the new byte size is 16 bytes aligned when shrink.
///
/// The byte size itself is not required to be 16 bytes aligned, the components beyond the byte size
/// are always zero, the constraints above are to keep this.
///
/// This is suitable for the large data that is written in whole. It can not be used under the
/// combined buffer, which writes the sub buffers by arbitrary offsets.
///
/// The behavior mirrors the gpu buffer implementation: the write is issued to the queue, and the
/// resize, relocation and copy are recorded in the given encoder. The clone is a ref clone, the
/// resize is visible to all clones.
#[derive(Clone)]
pub struct RgbaTextureAsReadonlyStorageBuffer {
  internal: Arc<RwLock<RgbaInternal>>,
  meta: HeapMeta,
  queue: GPUQueue,
}

struct RgbaInternal {
  texture: TextureU32x4Heap,
  byte_size: u64,
}

fn assert_texel_aligned(value: u64, what: &str) {
  assert!(
    value.is_multiple_of(RGBA_TEXEL_BYTE_SIZE),
    "rgba texture as storage buffer: {what} must be 16 bytes aligned, got {value}"
  );
}

fn check_aligned_relocation(r: &BufferRelocate, src_byte_size: u64, dst_byte_size: u64) {
  check_relocation(r, src_byte_size, dst_byte_size);
  assert_texel_aligned(r.self_offset, "the relocation source offset");
  assert_texel_aligned(r.target_offset, "the relocation target offset");
  assert_texel_aligned(r.count, "the relocation byte count");
}

impl RgbaTextureAsReadonlyStorageBuffer {
  /// the content is zero initialized, panic if the size exceeds the limit
  pub fn new(
    gpu: &GPU,
    byte_size: u64,
    ty_desc: MaybeUnsizedValueType,
    limit: TexelExtent,
    label: &str,
  ) -> Self {
    let meta = HeapMeta::new(ty_desc, label, limit, RGBA_TEXEL_BYTE_SIZE);
    let extent = meta.required_extent_or_panic(byte_size);
    let texture = TextureU32x4Heap::new(extent, label, &gpu.device);
    if let Some(length) = meta.array_length(byte_size) {
      texture.write_array_length(&gpu.queue, length);
    }

    Self {
      internal: Arc::new(RwLock::new(RgbaInternal { texture, byte_size })),
      meta,
      queue: gpu.queue.clone(),
    }
  }
}

impl AbstractBuffer for RgbaTextureAsReadonlyStorageBuffer {
  fn byte_size(&self) -> u64 {
    self.internal.read().byte_size
  }

  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
    relocations: Option<&mut dyn Iterator<Item = BufferRelocate>>,
  ) -> bool {
    let Some(extent) = self.meta.required_extent(new_byte_size) else {
      return false;
    };

    let mut internal = self.internal.write();
    let old_byte_size = internal.byte_size;
    if new_byte_size < old_byte_size {
      // otherwise the last texel keeps the truncated data
      assert_texel_aligned(new_byte_size, "the shrink target size");
    }
    if new_byte_size == old_byte_size && relocations.is_none() {
      return true;
    }

    // the components beyond the byte size are always zero, so the grow can be done in place.
    let grow_in_place = relocations.is_none()
      && new_byte_size > old_byte_size
      && internal.texture.extent().contains(&extent);

    if !grow_in_place {
      let new_texture = TextureU32x4Heap::new(extent, &self.meta.label, device);
      // when shrink the new size is aligned, when grow the components beyond the old size in the
      // last texel are zero, so the whole texels can be copied in both cases. only the data part is
      // copied, see TextureU32x4Heap.
      let keep_count = old_byte_size
        .min(new_byte_size)
        .div_ceil(RGBA_TEXEL_BYTE_SIZE);
      internal
        .texture
        .copy_to(&new_texture, 0, 0, keep_count, encoder);

      // same as the gpu buffer, the relocation copies from the old content to the new texture
      if let Some(relocations) = relocations {
        for r in relocations {
          check_aligned_relocation(&r, old_byte_size, new_byte_size);
          internal.texture.copy_to(
            &new_texture,
            r.self_offset / RGBA_TEXEL_BYTE_SIZE,
            r.target_offset / RGBA_TEXEL_BYTE_SIZE,
            r.count / RGBA_TEXEL_BYTE_SIZE,
            encoder,
          );
        }
      }
      internal.texture = new_texture;
    }

    internal.byte_size = new_byte_size;
    if let Some(length) = self.meta.array_length(new_byte_size) {
      internal.texture.write_array_length(&self.queue, length);
    }
    true
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    let internal = self.internal.read();
    let count = content.len() as u64;
    check_range(offset, count, internal.byte_size);
    assert_texel_aligned(offset, "the write offset");
    let end = offset + count;
    if end != internal.byte_size {
      assert_texel_aligned(end, "the write end, unless it equals the byte size,");
    }

    let texel_offset = offset / RGBA_TEXEL_BYTE_SIZE;
    let aligned_len = content.len() - content.len() % RGBA_TEXEL_BYTE_SIZE as usize;
    internal
      .texture
      .write(queue, texel_offset, &content[..aligned_len]);

    // the last partial texel is padded with zero
    let tail = &content[aligned_len..];
    if !tail.is_empty() {
      let mut texel = [0; RGBA_TEXEL_BYTE_SIZE as usize];
      texel[..tail.len()].copy_from_slice(tail);
      let tail_texel = texel_offset + aligned_len as u64 / RGBA_TEXEL_BYTE_SIZE;
      internal.texture.write(queue, tail_texel, &texel);
    }
  }

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    let relocations: Vec<_> = iter.filter(|r| r.count > 0).collect();
    if relocations.is_empty() {
      return;
    }

    let internal = self.internal.read();
    let texture = &internal.texture;
    let byte_size = internal.byte_size;
    relocations
      .iter()
      .for_each(|r| check_aligned_relocation(r, byte_size, byte_size));

    // the texture can not be copied to itself, so the rows that cover the relocation sources are
    // copied into a snapshot first, this also handles the overlapped relocations.
    let texel = RGBA_TEXEL_BYTE_SIZE;
    let start = relocations.iter().map(|r| r.self_offset / texel).min();
    let end = relocations
      .iter()
      .map(|r| (r.self_offset + r.count) / texel);
    let (start, end) = (start.unwrap(), end.max().unwrap());

    let width = texture.extent().width as u64;
    let first_row = start / width;
    let row_count = end.div_ceil(width) - first_row;
    let snapshot_extent = TexelExtent {
      width: texture.extent().width,
      height: row_count as u32,
    };
    let snapshot = TextureU32x4Heap::new(snapshot_extent, "rgba texture relocation", device);

    let base = first_row * width;
    texture.copy_to(&snapshot, base, 0, row_count * width, encoder);

    for r in relocations {
      let src = r.self_offset / texel - base;
      snapshot.copy_to(
        texture,
        src,
        r.target_offset / texel,
        r.count / texel,
        encoder,
      );
    }
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    let target = target
      .as_any()
      .downcast_ref::<Self>()
      .expect("the copy target must be RgbaTextureAsReadonlyStorageBuffer");
    assert!(
      !Arc::ptr_eq(&self.internal, &target.internal),
      "the copy target must not be self"
    );

    let src = self.internal.read();
    let dst = target.internal.read();
    check_range(self_offset, count, src.byte_size);
    check_range(target_offset, count, dst.byte_size);
    assert_texel_aligned(self_offset, "the copy source offset");
    assert_texel_aligned(target_offset, "the copy target offset");
    // the components beyond the byte size of both side are zero, so the last partial texel can be
    // copied in whole
    let reach_both_end =
      self_offset + count == src.byte_size && target_offset + count == dst.byte_size;
    if !reach_both_end {
      assert_texel_aligned(
        count,
        "the copy byte count, unless the copy reaches the end of both side,",
      );
    }

    src.texture.copy_to(
      &dst.texture,
      self_offset / RGBA_TEXEL_BYTE_SIZE,
      target_offset / RGBA_TEXEL_BYTE_SIZE,
      count.div_ceil(RGBA_TEXEL_BYTE_SIZE),
      encoder,
    );
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let internal = self.internal.read();
    bind_heap_texture_shader(bind_builder, &internal.texture.view, &self.meta.ty_desc, 4)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(&self.internal.read().texture.view);
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    None
  }
}
