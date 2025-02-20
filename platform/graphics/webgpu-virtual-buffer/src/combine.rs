use crate::*;

pub struct CombinedBufferAllocatorInternal {
  label: String,
  buffer: Option<GPUBuffer>,
  usage: BufferUsages,
  buffer_need_rebuild: bool,
  sub_buffer_u32_size_requirements: Vec<u32>,
  sub_buffer_allocation_u32_offset: Vec<u32>,
  header_length: u32,
}

impl CombinedBufferAllocatorInternal {
  /// label must unique across binding
  pub fn new(label: impl Into<String>, usage: BufferUsages) -> Self {
    Self {
      label: label.into(),
      buffer: None,
      buffer_need_rebuild: true,
      sub_buffer_u32_size_requirements: Default::default(),
      sub_buffer_allocation_u32_offset: Default::default(),
      header_length: 0,
      usage,
    }
  }
  pub fn expect_buffer(&self) -> &GPUBuffer {
    let err = "merged buffer not yet build";
    assert!(!self.buffer_need_rebuild, "{err}");
    self.buffer.as_ref().expect(err)
  }

  pub fn allocate(&mut self, sub_buffer_u32_size: u32) -> usize {
    self.buffer_need_rebuild = true;
    let buffer_index = self.sub_buffer_u32_size_requirements.len();
    self
      .sub_buffer_u32_size_requirements
      .push(sub_buffer_u32_size);

    buffer_index
  }

  pub fn rebuild(&mut self, gpu: &GPU) {
    if !self.buffer_need_rebuild && self.buffer.is_some() {
      return;
    }

    let sub_buffer_allocation_u32_offset = self
      .sub_buffer_u32_size_requirements
      .iter()
      .scan(0, |offset, size| {
        let o = *offset;
        *offset += size;
        Some(o)
      })
      .collect::<Vec<_>>();

    let combine_buffer_count = self.sub_buffer_u32_size_requirements.len() as u32;
    let header_size = combine_buffer_count + 1;
    let data_size = self.sub_buffer_u32_size_requirements.iter().sum::<u32>();

    let full_size_requirement_in_u32 = header_size + data_size;

    let buffer = {
      let usage = self.usage | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
      let init_size = NonZeroU64::new(full_size_requirement_in_u32 as u64 * 4).unwrap();
      let init = BufferInit::Zeroed(init_size);
      let desc = GPUBufferDescriptor {
        size: init.size(),
        usage,
      };

      GPUBuffer::create(&gpu.device, init, usage)
    };

    create_gpu_read_write_storage(full_size_requirement_in_u32 as usize, &gpu.device);

    // write header
    gpu
      .queue
      .write_buffer(buffer.gpu(), 0, cast_slice(&[combine_buffer_count]));
    gpu.queue.write_buffer(
      buffer.gpu(),
      4,
      cast_slice(&self.sub_buffer_u32_size_requirements),
    );

    // old data movement
    if let Some(old_buffer) = &self.buffer {
      let mut encoder = gpu.create_encoder();
      for (i, offset) in self.sub_buffer_allocation_u32_offset.iter().enumerate() {
        let size = self.sub_buffer_u32_size_requirements[i];
        let new_offset = sub_buffer_allocation_u32_offset[i] + header_size;
        let source_offset = offset + self.header_length;
        encoder.copy_buffer_to_buffer(
          old_buffer.gpu(),
          (source_offset * 4) as u64,
          buffer.gpu(),
          (new_offset * 4) as u64,
          (size * 4) as u64,
        );
      }
      gpu.submit_encoder(encoder);
    }

    self.buffer = Some(buffer);
    self.header_length = header_size;
    self.sub_buffer_allocation_u32_offset = sub_buffer_allocation_u32_offset;
    self.buffer_need_rebuild = false;
  }

  pub fn resize(&mut self, index: usize, new_u32_size: u32) {
    self.sub_buffer_u32_size_requirements[index] = new_u32_size;
    self.buffer_need_rebuild = true;
  }

  pub fn write_content(&mut self, index: usize, content: &[u8], queue: &GPUQueue) {
    let buffer = self.expect_buffer();
    let offset = self.sub_buffer_allocation_u32_offset[index];
    let offset = (offset * 4) as u64;
    queue.write_buffer(buffer.gpu(), offset, content);
  }

  pub fn get_sub_gpu_buffer_view(&self, index: usize) -> GPUBufferView {
    let buffer = self.expect_buffer().clone();

    let offset = self.sub_buffer_allocation_u32_offset[index];
    let offset = (offset + self.header_length) as u64 * 4;
    let size = self.sub_buffer_u32_size_requirements[index] as u64 * 4;
    let range = GPUBufferViewRange {
      offset,
      size: Some(NonZeroU64::new(size).unwrap()),
    };

    buffer.create_view(&range)
  }

  pub fn bind_shader<T: ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
    index: usize,
  ) -> ShaderPtrOf<T> {
    let ptr = self.bind_shader_impl(bind_builder, registry, index, T::maybe_unsized_ty());
    T::create_view_from_raw_ptr(ptr)
  }

  #[inline(never)]
  pub fn bind_shader_impl(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
    index: usize,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedShaderPtr {
    let label = &self.label;

    #[derive(Clone)]
    struct ShaderMeta {
      pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
      pub array: ShaderPtrOf<[u32]>,
    }

    let (_, array) = registry
      .dynamic_anything
      .raw_entry_mut()
      .from_key(label)
      .or_insert_with(|| {
        let buffer = self.expect_buffer();
        let buffer = bind_builder.bind_by(&buffer);
        let meta = ShaderMeta {
          meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(
            VirtualShaderTypeLayout::Std430,
          ))),
          array: buffer,
        };
        (label.to_string(), Box::new(meta))
      });

    let ShaderMeta { meta, array } = array.downcast_ref::<ShaderMeta>().unwrap().clone();

    let ty = meta.write().register_ty(ty_desc);

    let base_offset = self.sub_buffer_allocation_u32_offset[index] + self.header_length;

    let ptr = U32BufferLoadStoreSourceWithType {
      ptr: U32BufferLoadStoreSource {
        array,
        offset: val(base_offset),
      },
      ty,
      meta,
    };
    Box::new(ptr)
  }
}
