use crate::*;

/// Wrap a buffer binding source to bind it with dynamic offset.
///
/// The inner view's range decides the base offset and the size of the bindgroup entry, the
/// `offset` is applied when setting the bindgroup. The bindgroup cache key not contains the
/// offset, so changing offset will reuse the same bindgroup.
///
/// The inner view should have explicit size(or the offset can only be zero), and the offset must
/// be aligned to min_uniform_buffer_offset_alignment or min_storage_buffer_offset_alignment.
///
/// The dynamic offset binding has different binding layout, so the wrapper type should be used in
/// both shader side and pass side. The offset is ignored in shader side.
#[derive(Clone)]
pub struct DynamicOffsetBinding<T> {
  pub inner: T,
  pub offset: DynamicOffset,
}

impl<T> DynamicOffsetBinding<T> {
  pub fn new(inner: T, offset: DynamicOffset) -> Self {
    Self { inner, offset }
  }
}

impl<T: ShaderBindingProvider> ShaderBindingProvider for DynamicOffsetBinding<T> {
  type Node = T::Node;
  type ShaderInstance = T::ShaderInstance;

  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    self.inner.create_instance(node)
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut desc = self.inner.binding_desc();
    desc.has_dynamic_offset = true;
    desc
  }
}

impl<T> AbstractBindingSource for DynamicOffsetBinding<T>
where
  T: CacheAbleBindingSource + ShaderBindingProvider,
{
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.check_binding_layout(|| self.binding_desc());
    ctx.bind_dyn_with_dynamic_offset(self.inner.get_binding_build_source(), self.offset);
  }
}

pub fn dynamic_offset_stride(item_byte_size: u64, alignment: u32) -> u64 {
  item_byte_size.div_ceil(alignment as u64) * alignment as u64
}

/// A uniform buffer holding multiple T, each of them could be selected by dynamic offset binding.
#[derive(Clone)]
pub struct UniformBufferDynamicOffsetArray<T: Std140> {
  /// the view only covers the first item
  view: UniformBufferDataView<T>,
  stride: u64,
  count: u32,
}

impl<T: Std140> UniformBufferDynamicOffsetArray<T> {
  pub fn create(device: &GPUDevice, count: u32, debug_label: &str) -> Self {
    assert!(count > 0);
    let alignment = device.limits().min_uniform_buffer_offset_alignment;
    let item_size = std::mem::size_of::<T>() as u64;
    let stride = dynamic_offset_stride(item_size, alignment);

    let usage = gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST;
    let size = NonZeroU64::new(stride * count as u64).unwrap();
    let desc = GPUBufferDescriptor { size, usage };

    let gpu = GPUBuffer::create(device, Some(debug_label), BufferInit::Zeroed(size), usage);
    let gpu =
      GPUBufferResource::create_with_raw(gpu, desc, device).create_view(GPUBufferViewRange {
        offset: 0,
        size: NonZeroU64::new(item_size),
      });

    Self {
      view: UniformBufferDataView {
        gpu,
        phantom: PhantomData,
      },
      stride,
      count,
    }
  }

  pub fn count(&self) -> u32 {
    self.count
  }

  /// in bytes
  pub fn stride(&self) -> u64 {
    self.stride
  }

  pub fn write_at(&self, queue: &gpu::Queue, index: u32, data: &T) {
    assert!(index < self.count);
    let offset = index as u64 * self.stride;
    queue.write_buffer(&self.view.gpu.resource.gpu, offset, data.as_bytes());
  }

  /// the returned binding could be used in both shader side and pass side.
  pub fn bind_at(&self, index: u32) -> DynamicOffsetBinding<UniformBufferDataView<T>> {
    assert!(index < self.count);
    let offset = index as u64 * self.stride;
    DynamicOffsetBinding::new(self.view.clone(), offset as DynamicOffset)
  }
}

#[pollster::test]
async fn test_uniform_dynamic_offset_binding() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let count = 3;
  let uniforms = UniformBufferDynamicOffsetArray::<Vec4<u32>>::create(&gpu.device, count, "test");
  for i in 0..count {
    uniforms.write_at(&gpu.queue, i, &Vec4::new(i * 10 + 1, i, 0, 0));
  }
  let output = create_gpu_read_write_storage::<[u32]>(
    ZeroedArrayByArrayLength(count as usize),
    &gpu,
    "output",
  );

  let pipeline = {
    let mut cx = compute_shader_builder(&gpu).with_config_work_group_size(1);
    let input = cx.bind_by(&uniforms.bind_at(0)).load();
    let output = cx.bind_by(&output);
    output.index(input.y()).store(input.x());
    cx.create_compute_pipeline(&gpu, "dynamic offset test")
      .unwrap()
  };

  let cached_binding_count = || {
    gpu
      .device
      .get_binding_cache()
      .cache
      .read()
      .cached_binding_count()
  };
  let binding_count_before = cached_binding_count();

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    for i in 0..count {
      let mut binding = BindingBuilder::default();
      binding.setup_checking_layout(&pipeline.bg_layouts);
      binding
        .with_bind(&uniforms.bind_at(i))
        .with_bind(&output)
        .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
      pass.dispatch_workgroups(1, 1, 1);
    }
  });

  assert_eq!(cached_binding_count(), binding_count_before + 1);

  let result = encoder.read_buffer(&gpu.device, &output);
  gpu.submit_encoder(encoder);

  let result = result.await.unwrap();
  let result = <[u32]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
  assert_eq!(result, vec![1, 11, 21]);
}
