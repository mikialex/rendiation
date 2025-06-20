use crate::*;

pub struct AtomicImageDowngrade {
  buffer: StorageBufferDataView<[DeviceAtomic<u32>]>,
  size: UniformBufferDataView<Vec4<u32>>,
}

impl AtomicImageDowngrade {
  pub fn new(device: &GPUDevice, size: Size, layer_count: u32) -> Self {
    assert!(layer_count > 0);
    let (width, height) = size.into_usize();
    let init = ZeroedArrayByArrayLength(width * height * layer_count as usize);
    let area = width * height;
    Self {
      buffer: create_gpu_read_write_storage(init, device),
      size: create_uniform(Vec4::new(width as u32, area as u32, layer_count, 0), device),
    }
  }

  pub fn clear(encoder: &mut GPUCommandEncoder, value: u32) {
    todo!()
  }

  pub fn build(&self, builder: &mut ShaderBindGroupBuilder) -> AtomicImageInvocationDowngrade {
    let info = builder.bind_by(&self.size).load();
    AtomicImageInvocationDowngrade {
      buffer: builder.bind_by(&self.buffer),
      width: info.x(),
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
  area: Node<u32>,
  layer_count: Node<u32>,
}

impl AtomicImageInvocationDowngrade {
  pub fn layer_count(&self) -> Node<u32> {
    self.layer_count
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
