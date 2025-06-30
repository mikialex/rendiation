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

fn depth_reducer(reverse_depth: bool) -> &'static dyn QuadReducer<f32> {
  if reverse_depth {
    &MaxReducer as &dyn QuadReducer<f32>
  } else {
    &MinReducer
  }
}

pub fn compute_hierarchy_depth_from_depth_texture(
  pass: &mut GPUComputePass,
  device: &GPUDevice,
  texture: &GPU2DTexture,
  reverse_depth: bool,
) {
  fast_down_sampling::<f32>(
    depth_reducer(reverse_depth),
    &CommonTextureFastDownSamplingSource::<f32, f32>::new(
      texture,
      |tex| Box::new(FirstChannelLoader(tex)),
      |tex| Box::new(SplatWriter(tex)),
    ),
    pass,
    device,
  );
}

///  Enlarge the output depth texture to make sure no depth info is discard.
pub fn next_pot_sizer(size: Size) -> Size {
  let (width, height) = size.into_usize();
  let width = width.next_power_of_two();
  let height = height.next_power_of_two();
  Size::from_usize_pair_min_one((width, height))
}

pub fn compute_pot_enlarged_hierarchy_depth(
  input_depth: GPUTextureView,
  output_target: &GPU2DTexture,
  cx: &mut FrameCtx,
  device: &GPUDevice,
  reverse_depth: bool,
) {
  assert_eq!(
    input_depth.resource.desc.format,
    TextureFormat::Depth32Float
  );
  assert_eq!(output_target.desc.sample_count, 1);
  assert_eq!(output_target.desc.format, TextureFormat::R32Float);
  let size = next_pot_sizer(input_depth.size_assume_2d());
  assert_eq!(size.width_usize() as u32, output_target.width());
  assert_eq!(size.height_usize() as u32, output_target.height());

  if input_depth.resource.desc.sample_count == 1 {
    // do a full frame copy from the input.
    // todo, this can be improved to save bandwidth cost like how we did in multi sample

    struct CopyDepthFrame {
      source: GPU2DDepthTextureView,
    }

    impl ShaderHashProvider for CopyDepthFrame {
      shader_hash_type_id! {}
    }

    impl ShaderPassBuilder for CopyDepthFrame {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.source);
      }
    }

    impl GraphicsShaderProvider for CopyDepthFrame {
      fn build(&self, builder: &mut rendiation_shader_api::ShaderRenderPipelineBuilder) {
        builder.fragment(|builder, binding| {
          let source = binding.bind_by(&self.source);

          let position = builder.query::<FragmentPosition>().into_u32().xy();
          let value: Node<f32> = source.load_texel(position, val(0));
          builder.store_fragment_out(0, value)
        })
      }
    }

    let output_target_base = output_target.create_view(TextureViewDescriptor {
      mip_level_count: Some(1),
      ..Default::default()
    });

    pass("copy depth to hierarchy depth base")
      .with_color(
        &RenderTargetView::Texture(output_target_base.clone()),
        load_and_store(),
      )
      .render_ctx(cx)
      .by(
        &mut CopyDepthFrame {
          source: input_depth.try_into().unwrap(),
        }
        .draw_quad(),
      );

    let mut pass = cx.encoder.begin_compute_pass();

    compute_hierarchy_depth_from_depth_texture(&mut pass, device, output_target, reverse_depth);
  } else {
    let input_depth = GPU2DMultiSampleDepthTextureView::try_from(input_depth).unwrap();

    let internal = CommonTextureFastDownSamplingSource::new(
      output_target,
      |tex| Box::new(FirstChannelLoader(tex)),
      |tex| Box::new(SplatWriter(tex)),
    );

    let mut pass = cx.encoder.begin_compute_pass();
    fast_down_sampling(
      depth_reducer(reverse_depth),
      &MsaaDepthFastDownSamplingSource {
        source: input_depth.clone(),
        first_pass_base_write: internal
          .base
          .clone()
          .texture
          .into_storage_texture_view_writeonly()
          .unwrap(),
        internal,
        reverse_depth,
      },
      &mut pass,
      device,
    );
  }

  struct MsaaDepthFastDownSamplingSource {
    source: GPU2DMultiSampleDepthTextureView,
    first_pass_base_write: StorageTextureViewWriteonly2D,
    internal: CommonTextureFastDownSamplingSource<f32, f32>,
    reverse_depth: bool,
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
        reverse_depth: self.reverse_depth,
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
    reverse_depth: bool,
  }

  impl FastDownSamplingIOFirstStageInvocation<f32> for MsaaDownSampleFirstPass {
    fn get_root_loader_with_possible_write(&self) -> Box<dyn SourceImageLoader<f32>> {
      Box::new(MultisampleDepthInitLoader {
        ms_depth: self.msaa_input,
        mip_0: self.base_level,
        scale: self.msaa_input.texture_dimension_2d(None).into_f32()
          / self.base_level.texture_dimension_2d(None).into_f32(),
        reducer: depth_reducer(self.reverse_depth),
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
    reducer: &'static dyn QuadReducer<f32>,
  }

  impl SourceImageLoader<f32> for MultisampleDepthInitLoader {
    fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
      let depth_coord = coord.into_f32() * self.scale;
      let depth_coord = depth_coord.round().into_u32();

      let d1 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 0);
      let d2 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 1);
      let d3 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 2);
      let d4 = self.ms_depth.load_texel_multi_sample_index(depth_coord, 3);

      let v = self.reducer.reduce([d1, d2, d3, d4]);
      self.mip_0.write_texel(coord, v.splat());
      v
    }
  }
}
