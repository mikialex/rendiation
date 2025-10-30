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
  internal: Arc<RwLock<FastHashMap<Vec2<u32>, UniformBufferDataView<RenderPassGPUInfoData>>>>,
}

impl PassInfoPool {
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
      UniformBufferDataView::create(device, pass_info)
    });

    pass_info.clone()
  }
}
