use crate::*;

pub fn fast_down_sampling_generate_mipmap(
  pass: &mut GPUComputePass,
  device: &GPUDevice,
  texture: &GPU2DTexture,
) {
  fast_down_sampling::<Vec4<f32>>(
    &MipMapReducer,
    &CommonTextureFastDownSamplingSource::<f32, Vec4<f32>>::new(
      texture,
      |tex| Box::new(tex),
      |tex| Box::new(tex),
    ),
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
  let internal = CommonTextureFastDownSamplingSource::new(
    output_target,
    |tex| Box::new(FirstChannelLoader(tex)),
    |tex| Box::new(SplatWriter(tex)),
  );

  fast_down_sampling(
    &MaxReducer,
    &MsaaDepthFastDownSamplingSource {
      source: input_multi_sampled_depth.clone(),
      first_pass_base_write: internal
        .base
        .clone()
        .texture
        .into_storage_texture_view_writeonly()
        .unwrap(),
      internal,
    },
    pass,
    device,
  );

  struct MsaaDepthFastDownSamplingSource {
    source: GPU2DMultiSampleDepthTextureView,
    first_pass_base_write: StorageTextureViewWriteonly2D,
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
        base_level: cx.bind_by(&self.first_pass_base_write),
        levels: self
          .internal
          .first_pass_writes
          .clone()
          .map(|v| cx.bind_by(&v)),
      })
    }

    fn bind_first_stage_pass(&self, cx: &mut BindingBuilder) {
      cx.bind(&self.source);
      cx.bind(&self.first_pass_base_write);
      for level in self.internal.first_pass_writes.iter() {
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
    base_level: BindingNode<ShaderStorageTextureW2D>,
    levels: [BindingNode<ShaderStorageTextureW2D>; 6],
  }

  impl FastDownSamplingIOFirstStageInvocation<f32> for MsaaDownSampleFirstPass {
    fn get_root_loader_with_possible_write(&self) -> Box<dyn SourceImageLoader<f32>> {
      Box::new(MultisampleDepthInitLoader {
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

  struct MultisampleDepthInitLoader {
    mip_0: BindingNode<ShaderStorageTextureW2D>,
    ms_depth: BindingNode<ShaderMultiSampleDepthTexture2D>,
    scale: Node<Vec2<f32>>,
  }

  impl SourceImageLoader<f32> for MultisampleDepthInitLoader {
    fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
      let depth_coord = coord.into_f32() * self.scale;
      let depth_coord = depth_coord.round().into_u32();

      let d1 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 0);
      let d2 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 1);
      let d3 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 2);
      let d4 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 3);

      let v = MaxReducer.reduce([d1, d2, d3, d4]);
      self.mip_0.write_texel(coord, v.splat());
      v
    }
  }
}
