use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, PartialEq, ShaderStruct, Default)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}

impl RenderPassGPUInfoData {
  pub fn new(texel_size: Vec2<f32>, buffer_size: Vec2<f32>) -> Self {
    Self {
      texel_size,
      buffer_size,
      ..Default::default()
    }
  }
}

#[derive(Default, Clone)]
pub struct PassInfoPool {
  current_generation_idx: u64,
  internal: Arc<RwLock<FastHashMap<Vec2<u32>, PassPoolItem>>>,
}

type PassPoolItem = (UniformBufferDataView<RenderPassGPUInfoData>, u64);

impl PassInfoPool {
  /// every 15 ticks we check and remove any item that not been accessed in 3 last ticks
  pub fn tick(&mut self) {
    self.current_generation_idx += 1;

    if self.current_generation_idx.is_multiple_of(15) {
      let mut internal = self.internal.write();
      internal.retain(|_, v| self.current_generation_idx - v.1 <= 3);
    }
  }

  pub fn get_pass_info(
    &self,
    viewport_physical_pixel_size: Vec2<f32>,
    device: &GPUDevice,
  ) -> UniformBufferDataView<RenderPassGPUInfoData> {
    let key = viewport_physical_pixel_size.map(|v| v.to_bits());

    let mut internal = self.internal.write();
    let pass_info = internal.entry(key).or_insert_with(|| {
      let buffer_size = viewport_physical_pixel_size;

      let pass_info = RenderPassGPUInfoData::new(buffer_size.map(|v| 1.0 / v), buffer_size);
      (
        UniformBufferDataView::create(device, pass_info),
        self.current_generation_idx,
      )
    });

    pass_info.1 = self.current_generation_idx;

    pass_info.0.clone()
  }
}
