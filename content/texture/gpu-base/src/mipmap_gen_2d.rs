use crate::*;

// https://github.com/BabylonJS/Babylon.js/blob/d25bc29091/packages/dev/core/src/Engines/WebGPU/webgpuTextureHelper.ts

/// Mipmap generation is not supported in webgpu api for now, at least in mvp as far as i known.
/// It's also useful to provide customizable reducer / gen method for proper usage.
///
/// layer reduce logic, layer by layer.
/// input previous layer, generate next layer.
/// `current` is the layer's current writing pixel coordinate, range from 0. to 1.
pub trait Mipmap2dReducer: Send + Sync {
  fn reduce(
    &self,
    source: BindingNode<ShaderTexture2D>,
    sampler: BindingNode<ShaderSampler>,
    current: Node<Vec2<f32>>,
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

impl<T: Mipmap2dReducer> Mipmap2dReducerImpl for T {}
pub trait Mipmap2dReducerImpl: Mipmap2dReducer + Sized {
  fn generate(&self, ctx: &GPU, encoder: &mut GPUCommandEncoder, texture: &GPU2DTexture) {
    for write_level in 1..texture.desc.mip_level_count {
      let write_view: GPU2DTextureView = texture
        .create_view(TextureViewDescriptor {
          base_mip_level: write_level,
          mip_level_count: Some(1),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let read_level = write_level - 1;
      let read_view = texture
        .create_view(TextureViewDescriptor {
          base_mip_level: read_level,
          mip_level_count: Some(1),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let mut task = Mipmap2DGeneratorTask {
        view: read_view,
        reducer: self,
      }
      .draw_quad();

      pass("mip-gen-2d")
        .with_color(&RenderTargetView::from(write_view), load())
        .render(encoder, ctx)
        .by(&mut task);
    }
  }

  /// It's useful to generate cube faces use same method like 2d.
  /// even it's not correct from perspective of spherical filtering.
  fn generate_cube_faces(
    &self,
    ctx: &GPU,
    encoder: &mut GPUCommandEncoder,
    texture: &GPUCubeTexture,
  ) {
    for write_level in 1..texture.desc.mip_level_count {
      for face in 0..texture.desc.size.depth_or_array_layers {
        let write_view: GPU2DTextureView = texture
          .create_view(TextureViewDescriptor {
            base_mip_level: write_level,
            mip_level_count: Some(1),
            base_array_layer: face,
            // it defaults to None which defaults to cube type
            dimension: Some(TextureViewDimension::D2),
            ..Default::default()
          })
          .try_into()
          .unwrap();

        let read_level = write_level - 1;
        let read_view = texture
          .create_view(TextureViewDescriptor {
            base_mip_level: read_level,
            mip_level_count: Some(1),
            base_array_layer: face,
            ..Default::default()
          })
          .try_into()
          .unwrap();

        let mut task = Mipmap2DGeneratorTask {
          view: read_view,
          reducer: self,
        }
        .draw_quad();

        pass("mip-gen-cube-face")
          .with_color(&RenderTargetView::from(write_view), load())
          .render(encoder, ctx)
          .by(&mut task);
      }
    }
  }
}

pub struct DefaultMipmapReducer;

impl Mipmap2dReducer for DefaultMipmapReducer {
  #[rustfmt::skip]
  fn reduce(
    &self,
    source: BindingNode<ShaderTexture2D>,
    sampler: BindingNode<ShaderSampler>,
    current: Node<Vec2<f32>>,
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let mut r = source.sample_zero_level(sampler, current + texel_size * val(Vec2::new( 0.5,  0.5)));
    r        += source.sample_zero_level(sampler, current + texel_size * val(Vec2::new(-0.5, -0.5)));
    r        += source.sample_zero_level(sampler, current + texel_size * val(Vec2::new(-0.5,  0.5)));
    r        += source.sample_zero_level(sampler, current + texel_size * val(Vec2::new( 0.5, -0.5)));
    r / val(4.).splat()
  }
}

struct Mipmap2DGeneratorTask<'a> {
  view: GPU2DTextureView,
  reducer: &'a dyn Mipmap2dReducer,
}

impl ShaderPassBuilder for Mipmap2DGeneratorTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.view);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl ShaderHashProvider for Mipmap2DGeneratorTask<'_> {
  shader_hash_type_id! {Mipmap2DGeneratorTask<'static>}
}

impl GraphicsShaderProvider for Mipmap2DGeneratorTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let position = builder.query::<FragmentPosition>().xy();
      let buffer_size = builder.query::<RenderBufferSize>();
      let texel_size = builder.query::<TexelSize>() * val(0.5);
      let source = binding.bind_by(&self.view);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let result = self
        .reducer
        .reduce(source, sampler, position / buffer_size, texel_size);

      builder.store_fragment_out(0, result)
    })
  }
}
