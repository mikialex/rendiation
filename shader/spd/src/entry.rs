use std::array;

use crate::*;

pub fn fast_down_sampling_generate_mipmap(
  pass: &mut GPUComputePass,
  device: &GPUDevice,
  texture: &GPU2DTexture,
) {
  // level that exceeds will be clamped to max level
  let mips: [GPU2DTextureView; 13] = std::array::from_fn(|index| {
    texture
      .create_view(TextureViewDescriptor {
        base_mip_level: (index as u32).clamp(0, texture.mip_level_count() - 1),
        mip_level_count: Some(1),
        base_array_layer: 0,
        ..Default::default()
      })
      .try_into()
      .unwrap()
  });

  fast_down_sampling::<Vec4<f32>>(
    &MipMapReducer,
    &CommonTextureFastDownSamplingSource::<f32, Vec4<f32>> {
      target: texture.clone(),
      levels: mips,
      texel_to_reduce_unit: read_all,
      reduce_unit_to_texel: write_all,
    },
    pass,
    device,
  );
}

pub fn compute_hierarchy_depth_from_multi_sample_depth_texture(
  input_multi_sampled_depth: &GPU2DMultiSampleDepthTextureView,
  output_target: &GPU2DTexture,
  pass: &mut GPUComputePass,
  device: &GPUDevice,
) {
  // level that exceeds will be clamped to max level
  let mips: [GPU2DTextureView; 13] = std::array::from_fn(|index| {
    output_target
      .create_view(TextureViewDescriptor {
        base_mip_level: (index as u32).clamp(0, output_target.mip_level_count() - 1),
        mip_level_count: Some(1),
        base_array_layer: 0,
        ..Default::default()
      })
      .try_into()
      .unwrap()
  });

  fast_down_sampling(
    &MaxReducer,
    &MsaaDepthFastDownSamplingSource {
      source: input_multi_sampled_depth.clone(),
      internal: CommonTextureFastDownSamplingSource {
        target: output_target.clone(),
        levels: mips,
        texel_to_reduce_unit: |tex| Box::new(FirstChannelLoader(tex)),
        reduce_unit_to_texel: |tex| Box::new(SplatWriter(tex)),
      },
    },
    pass,
    device,
  );

  struct MsaaDepthFastDownSamplingSource {
    source: GPU2DMultiSampleDepthTextureView,
    internal: CommonTextureFastDownSamplingSource<f32, f32>,
  }

  impl ShaderHashProvider for MsaaDepthFastDownSamplingSource {
    shader_hash_type_id! {}
  }

  impl FastDownSamplingIO<f32> for MsaaDepthFastDownSamplingSource {
    fn root_size(&self) -> (u32, u32) {
      self.internal.root_size()
    }

    fn mip_level_count(&self) -> u32 {
      self.internal.mip_level_count()
    }

    fn bind_first_stage_shader(
      &self,
      cx: &mut ShaderComputePipelineBuilder,
    ) -> Box<dyn FastDownSamplingIOFirstStageInvocation<f32>> {
      Box::new(MsaaDownSampleFirstPass {
        msaa_input: cx.bind_by(&self.source),
        base_level: cx.bind_by(
          &self.internal.levels[0]
            .clone()
            .into_storage_texture_view_writeonly()
            .unwrap(),
        ),
        levels: array::from_fn(|i| {
          cx.bind_by(
            &self.internal.levels[i + 1]
              .clone()
              .into_storage_texture_view_writeonly()
              .unwrap(),
          )
        }),
      })
    }

    fn bind_first_stage_pass(&self, cx: &mut BindingBuilder) {
      cx.bind(&self.source);
      for level in self.internal.levels.get(0..6).unwrap().iter() {
        cx.bind(level);
      }
    }

    fn bind_second_stage_shader(
      &self,
      cx: &mut ShaderComputePipelineBuilder,
    ) -> Box<dyn FastDownSamplingIOSecondStageInvocation<f32>> {
      self.internal.bind_second_stage_shader(cx)
    }

    fn bind_second_stage_pass(&self, cx: &mut BindingBuilder) {
      self.internal.bind_second_stage_pass(cx)
    }
  }

  struct MsaaDownSampleFirstPass {
    msaa_input: BindingNode<ShaderMultiSampleDepthTexture2D>,
    base_level:
      BindingNode<ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, f32>>,
    levels:
      [BindingNode<ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, f32>>; 6],
  }

  impl FastDownSamplingIOFirstStageInvocation<f32> for MsaaDownSampleFirstPass {
    fn get_root_loader_with_possible_write(&self) -> Box<dyn SourceImageLoader<f32>> {
      Box::new(MSDepthLoader {
        ms_depth: self.msaa_input,
        mip_0: self.base_level,
        scale: self.msaa_input.texture_dimension_2d(None).into_f32()
          / self.base_level.texture_dimension_2d(None).into_f32(),
      })
    }

    fn get_1_6_level_writer(&self, absolute_index: usize) -> Box<dyn SourceImageWriter<f32>> {
      Box::new(SplatWriter(self.levels[absolute_index - 1]))
    }
  }
}
