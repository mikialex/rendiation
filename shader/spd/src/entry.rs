use crate::*;

/// todo, hash, and fix hash for common pass
///
/// the root level data maybe not exist and computed from other source, this trait support this case
///
/// the other way to solve is to use another pass to writer the root data at the cost of extra bandwidth
pub trait RootLevelDispatcher<FO, V> {
  fn create_root_loader_with_possible_write(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
    mip_zero: BindingNode<
      ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, FO>,
    >,
  ) -> Box<dyn SourceImageLoader<V>>;
  fn bind_root_input(&self, cx: &mut BindingBuilder);
}

/// the target is a h depth texture, the size must under MAX_INPUT_SIZE.
pub fn compute_hierarchy_depth_from_multi_sample_depth_texture(
  input_multi_sampled_depth: &GPU2DMultiSampleDepthTextureView,
  output_target: &GPU2DTexture,
  pass: &mut GPUComputePass,
  device: &GPUDevice,
) {
  let input_size = input_multi_sampled_depth.resource.desc.size;
  let mip_level_count = output_target.desc.mip_level_count;

  // level that exceeds will be clamped to max level
  let mips: [GPU2DTextureView; 13] = std::array::from_fn(|index| {
    output_target
      .create_view(TextureViewDescriptor {
        base_mip_level: (index as u32).clamp(0, mip_level_count - 1),
        mip_level_count: Some(1),
        base_array_layer: 0,
        ..Default::default()
      })
      .try_into()
      .unwrap()
  });

  fast_down_sampling::<f32, f32>(
    &MaxReducer,
    input_multi_sampled_depth,
    (input_size.width, input_size.height),
    mip_level_count,
    &mips,
    |level| Box::new(FirstChannelLoader(level)),
    |level| Box::new(SplatWriter(level)),
    pass,
    device,
  );
}

impl RootLevelDispatcher<f32, f32> for GPU2DMultiSampleDepthTextureView {
  fn create_root_loader_with_possible_write(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
    mip_zero: BindingNode<
      ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, f32>,
    >,
  ) -> Box<dyn SourceImageLoader<f32>> {
    let ms_depth = cx.bind_by(self);
    Box::new(MSDepthLoader {
      ms_depth,
      mip_0: mip_zero,
      scale: ms_depth.texture_dimension_2d(None).into_f32()
        / mip_zero.texture_dimension_2d(None).into_f32(),
    })
  }

  fn bind_root_input(&self, cx: &mut BindingBuilder) {
    cx.bind(self);
  }
}
