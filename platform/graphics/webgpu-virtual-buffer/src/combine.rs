use crate::*;

pub struct CombinedBufferAllocatorInternal {
  label: String,
  recording_bind_index_buffer: StorageBufferDataView<[u32]>,
  current_recording_count: u32,
  current_shader_recording_count: u32,
  gpu: GPU,
  buffer: Option<GPUBufferResourceView>,
  pending_writes: FastHashMap<usize, PendingWrites>,
  usage: BufferUsages,
  buffer_need_rebuild: bool,
  pub(crate) sub_buffer_u32_size_requirements: Vec<u32>,
  previous_sub_buffer_size: FastHashMap<usize, u32>,
  sub_buffer_allocation_u32_offset: Vec<u32>,
  pub(crate) layout: StructLayoutTarget,
  // use none for none atomic heap
  atomic: Option<ShaderAtomicValueType>,
  enable_debug_log: bool,
  /// if this allocator allow allocate writeable buffer
  pub(crate) readonly: bool,
}

#[derive(Default)]
struct PendingWrites {
  data: Vec<u8>,
  offset_sizes: Vec<(usize, usize, u64)>,
}

const MAX_BINDING_COUNT: usize = 1024;

impl CombinedBufferAllocatorInternal {
  /// label must unique across binding
  /// todo, add check
  pub fn new(
    gpu: &GPU,
    label: impl Into<String>,
    usage: BufferUsages,
    layout: StructLayoutTarget,
    atomic: Option<ShaderAtomicValueType>,
    readonly: bool,
  ) -> Self {
    Self {
      label: label.into(),
      buffer: None,
      buffer_need_rebuild: true,
      sub_buffer_u32_size_requirements: Default::default(),
      sub_buffer_allocation_u32_offset: Default::default(),
      previous_sub_buffer_size: Default::default(),
      recording_bind_index_buffer: create_gpu_read_write_storage::<[u32]>(
        ZeroedArrayByArrayLength(MAX_BINDING_COUNT),
        &gpu.device,
      ),
      pending_writes: Default::default(),
      current_recording_count: 0,
      current_shader_recording_count: 0,
      gpu: gpu.clone(),
      usage,
      layout,
      atomic,
      enable_debug_log: false,
      readonly,
    }
  }

  pub fn copy_buffer_to_buffer(
    &mut self,
    src_index: usize,
    target: &GPUBufferResourceView,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    let end = count as u32 + self_offset as u32;
    let bound = self.sub_buffer_u32_size_requirements[src_index] * 4;
    assert!(end <= bound);

    // webgpu spec don't allow copy buffer to buffer is same buffer, so we have to do a extra copy.
    // it's unsure if using compute shader do copy will faster than this approach.
    // todo, try compute shader
    let source = self.get_sub_gpu_buffer_view(src_index);

    let temp_buffer = create_gpu_buffer_zeroed(
      count,
      BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
      &self.gpu.device,
    );

    encoder.copy_buffer_to_buffer(
      source.resource.gpu(),
      self_offset + source.desc.offset,
      temp_buffer.gpu(),
      0,
      count,
    );

    encoder.copy_buffer_to_buffer(
      temp_buffer.gpu(),
      0,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  pub fn allocate(&mut self, sub_buffer_u32_size: u32) -> usize {
    self.buffer_need_rebuild = true;
    let buffer_index = self.sub_buffer_u32_size_requirements.len();
    self
      .sub_buffer_u32_size_requirements
      .push(sub_buffer_u32_size);

    buffer_index
  }

  fn check_rebuild(&mut self) {
    let gpu = &self.gpu;
    if !self.buffer_need_rebuild && self.buffer.is_some() {
      return;
    }

    // the sub buffer must be aligned to device limitation because user may directly
    // use the sub buffer as the storage/uniform binding
    let limits = &gpu.info.supported_limits;
    // for simplicity we use the maximum alignment, they are same on my machine.
    let bind_alignment_requirement_in_u32 = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    let sub_buffer_count = self.sub_buffer_u32_size_requirements.len() as u32;
    let header_size = sub_buffer_count * 2 + 1;

    let mut sub_buffer_allocation_u32_offset = Vec::with_capacity(sub_buffer_count as usize);
    let mut used_buffer_size_in_u32 = header_size;

    for sub_buffer_size in &self.sub_buffer_u32_size_requirements {
      // add padding
      used_buffer_size_in_u32 += align_offset(
        used_buffer_size_in_u32 as usize,
        bind_alignment_requirement_in_u32 as usize,
      ) as u32;
      sub_buffer_allocation_u32_offset.push(used_buffer_size_in_u32);
      used_buffer_size_in_u32 += *sub_buffer_size;
    }

    let full_size_requirement_in_u32 = used_buffer_size_in_u32;

    let buffer = {
      let usage = self.usage | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
      let init_size = NonZeroU64::new(full_size_requirement_in_u32 as u64 * 4).unwrap();
      let init = BufferInit::Zeroed(init_size);
      let desc = GPUBufferDescriptor {
        size: init.size(),
        usage,
      };

      let buffer = GPUBuffer::create(&gpu.device, self.label.as_str().into(), init, usage);
      let buffer = GPUBufferResource::create_with_raw(buffer, desc, &gpu.device);
      buffer.create_default_view()
    };

    // write header
    let new_buffer = buffer.resource.gpu();
    gpu
      .queue
      .write_buffer(new_buffer, 0, bytes_of(&sub_buffer_count));

    let offsets = cast_slice(&sub_buffer_allocation_u32_offset);
    gpu.queue.write_buffer(new_buffer, 4, offsets);
    let sizes = cast_slice(&self.sub_buffer_u32_size_requirements);
    gpu
      .queue
      .write_buffer(new_buffer, 4 + sub_buffer_count as u64 * 4, sizes);

    // old data movement
    if let Some(old_buffer) = &self.buffer {
      let mut encoder = gpu.create_encoder();
      for (i, source_offset) in self.sub_buffer_allocation_u32_offset.iter().enumerate() {
        let size = if let Some(size) = self.previous_sub_buffer_size.get(&i) {
          *size
        } else {
          self.sub_buffer_u32_size_requirements[i]
        };
        let new_offset = sub_buffer_allocation_u32_offset[i];
        encoder.copy_buffer_to_buffer(
          old_buffer.resource.gpu(),
          (source_offset * 4) as u64,
          new_buffer,
          (new_offset * 4) as u64,
          (size * 4) as u64,
        );
      }
      gpu.submit_encoder(encoder);
    }

    // write staged buffer
    for (i, pending_writes) in self.pending_writes.drain() {
      for (offset, size, write_offset) in pending_writes.offset_sizes {
        let data_to_write = &pending_writes.data[offset..offset + size];
        let write_offset = (sub_buffer_allocation_u32_offset[i] * 4) as u64 + write_offset;
        gpu
          .queue
          .write_buffer(new_buffer, write_offset, data_to_write);
      }
    }
    self.previous_sub_buffer_size.clear();

    self.buffer = Some(buffer);
    self.sub_buffer_allocation_u32_offset = sub_buffer_allocation_u32_offset;
    self.buffer_need_rebuild = false;
  }

  pub fn resize(&mut self, index: usize, new_u32_size: u32) {
    // only keep the first size, if resize invoke multiple times
    if !self.previous_sub_buffer_size.contains(&index) {
      self
        .previous_sub_buffer_size
        .insert(index, self.sub_buffer_u32_size_requirements[index]);
    }

    self.sub_buffer_u32_size_requirements[index] = new_u32_size;
    self.buffer_need_rebuild = true;
  }

  pub fn write_content(&mut self, index: usize, content: &[u8], offset: u64) {
    if self.buffer_need_rebuild {
      let pending = self.pending_writes.entry(index).or_default();

      let end = content.len() as u32 + offset as u32;
      let bound = self.sub_buffer_u32_size_requirements[index] * 4;
      assert!(end <= bound);

      pending
        .offset_sizes
        .push((pending.data.len(), content.len(), offset));
      pending.data.extend_from_slice(content);
    } else {
      let buffer = self.buffer.as_ref().unwrap();
      let b_offset = self.sub_buffer_allocation_u32_offset[index];
      let offset = (b_offset * 4) as u64 + offset;
      self
        .gpu
        .queue
        .write_buffer(buffer.resource.gpu(), offset, content);
    }
  }

  pub fn get_sub_gpu_buffer_view(&mut self, index: usize) -> GPUBufferResourceView {
    self.check_rebuild();
    let buffer = self.buffer.clone().unwrap();

    let offset = self.sub_buffer_allocation_u32_offset[index] as u64;
    let offset = offset * 4;
    let size = self.sub_buffer_u32_size_requirements[index] as u64 * 4;
    let range = GPUBufferViewRange {
      offset,
      size: Some(NonZeroU64::new(size).unwrap()),
    };

    buffer.resource.create_view(range)
  }

  #[inline(never)]
  pub fn bind_shader_impl(
    &mut self,
    bind_builder: &mut ShaderBindGroupBuilder,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedShaderPtr {
    let label = &self.label;

    #[derive(Clone)]
    struct ShaderMeta {
      pub meta: Arc<RwLock<ShaderU32StructMetaData>>,
      pub array: U32HeapHeapSource,
      pub bind_index_array: ShaderPtrOf<[u32]>,
    }

    let array = if let Some(r) = bind_builder.custom_states.get(label) {
      r
    } else {
      if self.enable_debug_log {
        println!("bind shader <{}>", self.label);
      }

      let heap_ty = if let Some(a_ty) = self.atomic {
        match a_ty {
          ShaderAtomicValueType::I32 => <[DeviceAtomic<i32>]>::ty(),
          ShaderAtomicValueType::U32 => <[DeviceAtomic<u32>]>::ty(),
        }
      } else {
        <[u32]>::ty()
      };

      let handle = bind_builder
        .binding_dyn(ShaderBindingDescriptor {
          should_as_storage_buffer_if_is_buffer_like: true,
          ty: heap_ty,
          // todo, if configured not readonly and all binding is readonly, this should be readonly for better performance
          writeable_if_storage: !self.readonly,
        })
        .using();

      let ptr = Box::new(handle);
      let array = if let Some(a_ty) = self.atomic {
        match a_ty {
          ShaderAtomicValueType::I32 => {
            U32HeapHeapSource::AtomicI32(<[DeviceAtomic<i32>]>::create_view_from_raw_ptr(ptr))
          }
          ShaderAtomicValueType::U32 => {
            U32HeapHeapSource::AtomicU32(<[DeviceAtomic<u32>]>::create_view_from_raw_ptr(ptr))
          }
        }
      } else {
        U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(ptr))
      };

      let meta = ShaderMeta {
        meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(self.layout))),
        array,
        bind_index_array: bind_builder.bind_by(&self.recording_bind_index_buffer),
      };
      self.current_shader_recording_count = 0;

      bind_builder
        .custom_states
        .insert(label.to_string(), Arc::new(meta));
      bind_builder.custom_states.get(label).unwrap()
    };

    let ShaderMeta {
      meta,
      array,
      bind_index_array,
    } = array.downcast_ref::<ShaderMeta>().unwrap().clone();

    // todo, should we put it at allocation time?
    meta.write().register_ty(&ty_desc);

    let buffer_bind_index = bind_index_array
      .index(self.current_shader_recording_count)
      .load();
    let offset = array.bitcast_read_u32_at(buffer_bind_index + val(1));

    self.current_shader_recording_count += 1;

    let ptr = U32HeapPtrWithType {
      ptr: U32HeapPtr { array, offset },
      ty: ty_desc.into_shader_single_ty(),
      bind_index: buffer_bind_index,
      meta,
    };
    Box::new(ptr)
  }

  pub fn bind_pass(&mut self, bind_builder: &mut BindingBuilder, index: usize) {
    self.check_rebuild();
    let buffer = self.buffer.as_ref().unwrap();
    let bounded = bind_builder
      .iter_groups()
      .flat_map(|g| g.iter_bounded())
      .any(|res| res.view_id == buffer.guid);

    if !bounded {
      if self.enable_debug_log {
        println!("bind res <{}>", self.label);
      }
      bind_builder.bind_dyn(buffer.get_binding_build_source());
      // new binding occurs, refresh binding index buffer
      self.recording_bind_index_buffer = create_gpu_read_write_storage::<[u32]>(
        ZeroedArrayByArrayLength(MAX_BINDING_COUNT),
        &self.gpu.device,
      );
      self.current_recording_count = 0;
      bind_builder.bind(&self.recording_bind_index_buffer);
    }
    self.gpu.queue.write_buffer(
      self.recording_bind_index_buffer.buffer.gpu(),
      self.current_recording_count as u64 * 4,
      bytes_of(&(index as u32)),
    );

    self.current_recording_count += 1;
  }
}
