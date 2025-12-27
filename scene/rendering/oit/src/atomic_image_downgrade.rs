use crate::*;

#[derive(Clone)]
pub struct AtomicImageDowngrade {
  buffer: StorageBufferDataView<[DeviceAtomic<u32>]>,
  size: UniformBufferDataView<Vec4<u32>>,
  size_: Size,
}

impl AtomicImageDowngrade {
  pub fn new(device: &GPUDevice, size: Size, layer_count: u32) -> Self {
    assert!(layer_count > 0);
    let (width, height) = size.into_usize();
    let init = ZeroedArrayByArrayLength(width * height * layer_count as usize);
    let area = width * height;
    Self {
      buffer: create_gpu_read_write_storage(init, device),
      size: create_uniform(
        Vec4::new(width as u32, area as u32, layer_count, height as u32),
        device,
      ),
      size_: size,
    }
  }

  pub fn size(&self) -> Size {
    self.size_
  }

  // todo, combine different layer's clear call?
  // wgpu does not have fill buffer cmd
  pub fn clear(&self, device: &GPUDevice, encoder: &mut GPUCommandEncoder, layer: u32, value: u32) {
    let clear_value = create_uniform(Vec4::new(value, layer, 0, 0), device);
    // cast to common buffer, as we not required atomic write in clear
    let buffer = StorageBufferDataView::<[u32]>::try_from_raw(self.buffer.gpu.clone()).unwrap();
    let workgroup_size = 256;
    encoder.compute_pass_scoped(|mut pass| {
      let hasher = shader_hasher_from_marker_ty!(BufferClear);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder.config_work_group_size(workgroup_size);
        let buffer = builder.bind_by(&buffer);
        let layer_area = builder.bind_by(&self.size).load().y();
        let config = builder.bind_by(&clear_value);
        let clear_value = config.load().x();
        let layer_idx = config.load().y();

        let offset = layer_idx * layer_area;

        let id = builder.global_invocation_id().x();
        if_by(id.less_than(layer_area), || {
          buffer.index(id + offset).store(clear_value);
        });

        builder
      });

      BindingBuilder::default()
        .with_bind(&buffer)
        .with_bind(&self.size)
        .with_bind(&clear_value)
        .setup_compute_pass(&mut pass, device, &pipeline);

      let area = self.size_.area() as u32;
      pass.dispatch_workgroups(area.div_ceil(workgroup_size), 1, 1);
    });
  }

  pub fn build(&self, builder: &mut ShaderBindGroupBuilder) -> AtomicImageInvocationDowngrade {
    let info = builder.bind_by(&self.size).load();
    AtomicImageInvocationDowngrade {
      buffer: builder.bind_by(&self.buffer),
      width: info.x(),
      height: info.w(),
      area: info.y(),
      layer_count: info.z(),
    }
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.size);
    builder.bind(&self.buffer);
  }
}

pub struct AtomicImageInvocationDowngrade {
  buffer: ShaderPtrOf<[DeviceAtomic<u32>]>,
  width: Node<u32>,
  height: Node<u32>,
  area: Node<u32>,
  layer_count: Node<u32>,
}

impl AtomicImageInvocationDowngrade {
  pub fn layer_count(&self) -> Node<u32> {
    self.layer_count
  }

  pub fn size(&self) -> Node<Vec2<u32>> {
    (self.width, self.height).into()
  }

  fn get_position(&self, position: Node<Vec2<u32>>, layer_idx: Node<u32>) -> Node<u32> {
    let x = position.x();
    let y = position.y();
    x + y * self.width + self.area * layer_idx
  }

  pub fn atomic_load(&self, position: Node<Vec2<u32>>, layer_idx: Node<u32>) -> Node<u32> {
    self
      .buffer
      .index(self.get_position(position, layer_idx))
      .atomic_load()
  }

  pub fn atomic_store(&self, position: Node<Vec2<u32>>, layer_idx: Node<u32>, value: Node<u32>) {
    self
      .buffer
      .index(self.get_position(position, layer_idx))
      .atomic_store(value)
  }

  pub fn atomic_min(
    &self,
    position: Node<Vec2<u32>>,
    layer_idx: Node<u32>,
    value: Node<u32>,
  ) -> Node<u32> {
    self
      .buffer
      .index(self.get_position(position, layer_idx))
      .atomic_min(value)
  }

  pub fn atomic_max(
    &self,
    position: Node<Vec2<u32>>,
    layer_idx: Node<u32>,
    value: Node<u32>,
  ) -> Node<u32> {
    self
      .buffer
      .index(self.get_position(position, layer_idx))
      .atomic_max(value)
  }
}
