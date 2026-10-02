use crate::*;

/// The texture as readonly storage buffer that keeps a full host backup.
///
/// The writes, copies and relocations are applied to the host backup, and the touched ranges are
/// recorded. The recorded ranges are merged and uploaded when the buffer is bound, or when
/// [Self::flush] is called. This is suitable for the large amount of fragmented writes, for example
/// the incrementally updated scene data, at the cost of a full host copy of the data.
///
/// Different from the gpu buffer implementation, the copies and relocations take effect
/// immediately instead of being recorded in the given encoder.
///
/// The clone is a ref clone, the resize is visible to all clones.
#[derive(Clone)]
pub struct TextureAsReadonlyStorageBufferWithHost {
  internal: Arc<RwLock<HostBackupInternal>>,
  meta: HeapMeta,
  gpu: GPU,
}

struct HostBackupInternal {
  /// the heap content including the header. the length never shrinks, and the part beyond the
  /// byte size is kept zeroed, so the grow within the length does not require upload.
  host: Vec<u8>,
  byte_size: u64,
  dirty: DirtyTexelRanges,
  /// the content is same as the host except the dirty part
  texture: TextureU32Heap,
}

impl TextureAsReadonlyStorageBufferWithHost {
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
    let mut internal = HostBackupInternal {
      host: vec![0; 4 + byte_size as usize],
      byte_size,
      dirty: Default::default(),
      texture: TextureU32Heap::new(extent, label, &gpu.device),
    };
    internal.write_header(&meta);

    Self {
      internal: Arc::new(RwLock::new(internal)),
      meta,
      gpu: gpu.clone(),
    }
  }

  /// upload the pending changes, this is called automatically when bind the buffer
  pub fn flush(&self) {
    self.internal.write().flush(&self.meta, &self.gpu);
  }
}

struct RelocationSnapshot {
  relocations: Vec<BufferRelocate>,
  /// the content starts from the min relocation source offset
  data: Vec<u8>,
  base: u64,
}

impl HostBackupInternal {
  fn write_host(&mut self, data: &[u8], host_offset: usize) {
    self.host[host_offset..host_offset + data.len()].copy_from_slice(data);
    self
      .dirty
      .push(host_offset / 4..(host_offset + data.len()) / 4);
  }

  fn write(&mut self, data: &[u8], byte_offset: u64) {
    check_range(byte_offset, data.len() as u64, self.byte_size);
    self.write_host(data, 4 + byte_offset as usize);
  }

  fn read(&self, byte_offset: u64, byte_count: u64) -> &[u8] {
    check_range(byte_offset, byte_count, self.byte_size);
    let start = 4 + byte_offset as usize;
    &self.host[start..start + byte_count as usize]
  }

  fn write_header(&mut self, meta: &HeapMeta) {
    if let Some(header) = meta.header(self.byte_size) {
      self.write_host(bytes_of(&header), 0);
    }
  }

  fn resize(&mut self, new_byte_size: u64, meta: &HeapMeta) {
    let old_byte_size = self.byte_size;
    if new_byte_size < old_byte_size {
      // keep the part beyond the byte size zeroed
      let truncated = 4 + new_byte_size as usize..4 + old_byte_size as usize;
      self.host[truncated.clone()].fill(0);
      self.dirty.push(truncated.start / 4..truncated.end / 4);
    }

    let len = 4 + new_byte_size as usize;
    if self.host.len() < len {
      self.host.resize(len, 0);
    }
    self.byte_size = new_byte_size;
    self.write_header(meta);
  }

  fn snapshot_relocations(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
  ) -> RelocationSnapshot {
    let relocations: Vec<_> = iter.filter(|r| r.count > 0).collect();
    let start = relocations.iter().map(|r| r.self_offset).min().unwrap_or(0);
    let end = relocations.iter().map(|r| r.self_offset + r.count).max();
    let end = end.unwrap_or(start);
    relocations
      .iter()
      .for_each(|r| check_range(r.self_offset, r.count, self.byte_size));

    RelocationSnapshot {
      data: self.read(start, end - start).to_vec(),
      relocations,
      base: start,
    }
  }

  fn apply_relocations(&mut self, snapshot: RelocationSnapshot) {
    for r in snapshot.relocations {
      let start = (r.self_offset - snapshot.base) as usize;
      self.write(
        &snapshot.data[start..start + r.count as usize],
        r.target_offset,
      );
    }
  }

  fn flush(&mut self, meta: &HeapMeta, gpu: &GPU) {
    // the host length is checked when resize
    let required = TexelExtent::required(self.host.len() as u64 / 4, meta.limit).unwrap();
    let reallocate = !self.texture.extent().contains(&required);
    if !reallocate && self.dirty.is_empty() {
      return;
    }

    if reallocate {
      let new_texture = TextureU32Heap::new(required, &meta.label, &gpu.device);
      // the old content is submitted immediately, so the later queue writes to the new texture
      // will not be overwritten by it.
      let mut encoder = gpu.create_encoder();
      let count = self.texture.extent().texel_count();
      self
        .texture
        .copy_to(&new_texture, 0, 0, count, &mut encoder);
      gpu.submit_encoder(encoder);
      self.texture = new_texture;
    }

    for range in self.dirty.take() {
      let data = &self.host[range.start * 4..range.end * 4];
      self.texture.write(&gpu.queue, range.start as u64, data);
    }
  }
}

impl AbstractBuffer for TextureAsReadonlyStorageBufferWithHost {
  fn byte_size(&self) -> u64 {
    self.internal.read().byte_size
  }

  fn resize_gpu(
    &mut self,
    _encoder: &mut GPUCommandEncoder,
    _device: &GPUDevice,
    new_byte_size: u64,
    relocations: Option<&mut dyn Iterator<Item = BufferRelocate>>,
  ) -> bool {
    if self.meta.required_extent(new_byte_size).is_none() {
      return false;
    }

    let mut internal = self.internal.write();
    // the relocation sources refer to the content before resize
    let relocations = relocations.map(|iter| internal.snapshot_relocations(iter));
    internal.resize(new_byte_size, &self.meta);
    if let Some(relocations) = relocations {
      internal.apply_relocations(relocations);
    }
    true
  }

  fn write(&self, content: &[u8], offset: u64, _queue: &GPUQueue) {
    self.internal.write().write(content, offset);
  }

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    _encoder: &mut GPUCommandEncoder,
    _device: &GPUDevice,
  ) {
    let mut internal = self.internal.write();
    let relocations = internal.snapshot_relocations(iter);
    internal.apply_relocations(relocations);
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    _encoder: &mut GPUCommandEncoder,
  ) {
    let target = target
      .as_any()
      .downcast_ref::<Self>()
      .expect("the copy target must be TextureAsReadonlyStorageBufferWithHost");
    assert!(
      !Arc::ptr_eq(&self.internal, &target.internal),
      "the copy target must not be self"
    );

    let src = self.internal.read();
    let data = src.read(self_offset, count);
    target.internal.write().write(data, target_offset);
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let internal = self.internal.read();
    bind_heap_texture_shader(bind_builder, &internal.texture.view, &self.meta.ty_desc)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.flush(&self.meta, &self.gpu);
    bind_builder.bind(&internal.texture.view);
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    None
  }
}
