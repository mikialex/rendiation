use crate::*;

/// The texture as readonly storage buffer that directly operates on the texture, without host
/// backup.
///
/// The behavior mirrors the gpu buffer implementation: the write is issued to the queue, and the
/// resize, relocation and copy are recorded in the given encoder. Each write is split into at most
/// 3 texture writes, so this is suitable for the data that is written in bulk, for example the per
/// frame data that is written once after creation. For the large amount of fragmented writes,
/// consider [TextureAsReadonlyStorageBufferWithHost].
///
/// The clone is a ref clone, the resize is visible to all clones.
#[derive(Clone)]
pub struct TextureAsReadonlyStorageBuffer {
  internal: Arc<RwLock<DirectInternal>>,
  meta: HeapMeta,
  queue: GPUQueue,
}

struct DirectInternal {
  texture: TextureU32Heap,
  byte_size: u64,
}

impl TextureAsReadonlyStorageBuffer {
  /// the content is zero initialized, panic if the size exceeds the limit
  pub fn new(
    gpu: &GPU,
    byte_size: u64,
    ty_desc: MaybeUnsizedValueType,
    limit: TexelExtent,
    label: &str,
  ) -> Self {
    let meta = HeapMeta::new(ty_desc, label, limit);
    let extent = meta.required_extent_or_panic(byte_size);
    let texture = TextureU32Heap::new(extent, label, &gpu.device);
    if let Some(length) = meta.array_length(byte_size) {
      texture.write_array_length(&gpu.queue, length);
    }

    Self {
      internal: Arc::new(RwLock::new(DirectInternal { texture, byte_size })),
      meta,
      queue: gpu.queue.clone(),
    }
  }
}

impl AbstractBuffer for TextureAsReadonlyStorageBuffer {
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
    if new_byte_size == old_byte_size && relocations.is_none() {
      return true;
    }

    // the part beyond the byte size is always zero, because the writes are checked within the
    // byte size and the shrink always reallocates, so the grow can be done in place.
    let grow_in_place = relocations.is_none()
      && new_byte_size > old_byte_size
      && internal.texture.extent().contains(&extent);

    if !grow_in_place {
      let new_texture = TextureU32Heap::new(extent, &self.meta.label, device);
      let keep_count = old_byte_size.min(new_byte_size) / 4;
      // only the data part is copied, see TextureU32Heap
      internal
        .texture
        .copy_to(&new_texture, 0, 0, keep_count, encoder);

      // same as the gpu buffer, the relocation copies from the old content to the new texture
      if let Some(relocations) = relocations {
        for r in relocations {
          check_relocation(&r, old_byte_size, new_byte_size);
          internal.texture.copy_to(
            &new_texture,
            r.self_offset / 4,
            r.target_offset / 4,
            r.count / 4,
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
    check_range(offset, content.len() as u64, internal.byte_size);
    internal.texture.write(queue, offset / 4, content);
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
    relocations
      .iter()
      .for_each(|r| check_relocation(r, internal.byte_size, internal.byte_size));

    // the texture can not be copied to itself, so the rows that cover the relocation sources are
    // copied into a snapshot first, this also handles the overlapped relocations.
    let start = relocations.iter().map(|r| r.self_offset / 4).min().unwrap();
    let end = relocations
      .iter()
      .map(|r| (r.self_offset + r.count) / 4)
      .max()
      .unwrap();

    let width = texture.extent().width as u64;
    let first_row = start / width;
    let row_count = end.div_ceil(width) - first_row;
    let snapshot_extent = TexelExtent {
      width: texture.extent().width,
      height: row_count as u32,
    };
    let snapshot = TextureU32Heap::new(snapshot_extent, "texture as buffer relocation", device);

    let base = first_row * width;
    texture.copy_to(&snapshot, base, 0, row_count * width, encoder);

    for r in relocations {
      let src = r.self_offset / 4 - base;
      snapshot.copy_to(texture, src, r.target_offset / 4, r.count / 4, encoder);
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
      .expect("the copy target must be TextureAsReadonlyStorageBuffer");
    assert!(
      !Arc::ptr_eq(&self.internal, &target.internal),
      "the copy target must not be self"
    );

    let src = self.internal.read();
    let dst = target.internal.read();
    check_range(self_offset, count, src.byte_size);
    check_range(target_offset, count, dst.byte_size);

    src.texture.copy_to(
      &dst.texture,
      self_offset / 4,
      target_offset / 4,
      count / 4,
      encoder,
    );
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let internal = self.internal.read();
    bind_heap_texture_shader(bind_builder, &internal.texture.view, &self.meta.ty_desc)
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
