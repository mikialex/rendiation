use __core::num::NonZeroU32;

use crate::*;

// https://github.com/BabylonJS/Babylon.js/blob/d25bc29091/packages/dev/core/src/Engines/WebGPU/webgpuTextureHelper.ts

/// Mipmap generation is not supported in webgpu api for now, at least in mvp as far as i known.
/// It's also useful to provide customizable reducer / gen method for proper usage.
///
pub struct Mipmap2DGenerator {
  pub reducer: Box<dyn Mipmap2dReducer>,
}

impl Mipmap2DGenerator {
  pub fn new(reducer: impl Mipmap2dReducer + 'static) -> Self {
    Self {
      reducer: Box::new(reducer),
    }
  }

  pub fn generate(&self, encoder: &mut GPUCommandEncoder, gpu: &GPU, texture: &GPU2DTexture) {
    for write_level in 1..texture.desc.mip_level_count {
      let mut desc = RenderPassDescriptorOwned::default();

      let write_view = texture
        .create_view(webgpu::TextureViewDescriptor {
          base_mip_level: write_level,
          mip_level_count: Some(NonZeroU32::new(1).unwrap()),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      desc.channels.push((
        webgpu::Operations {
          load: webgpu::LoadOp::Load,
          store: true,
        },
        RenderTargetView::Texture(write_view),
      ));

      let pass = encoder.begin_render_pass(desc);
      let pass = GPURenderPassCtx::new(pass, gpu);

      let read_level = write_level - 1;
      let read_view = texture
        .create_view(webgpu::TextureViewDescriptor {
          base_mip_level: read_level,
          mip_level_count: Some(NonZeroU32::new(1).unwrap()),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      Mipmap2DGeneratorTask {
        view: read_view,
        reducer: self.reducer.as_ref(),
      }
      .draw_quad();
    }
  }
}

/// layer reduce logic, layer by layer.
/// input previous layer, generate next layer.
/// target is the layer's current writing pixel coordinate.
pub trait Mipmap2dReducer {
  fn reduce(
    &self,
    previous_level: Node<ShaderTexture2D>,
    sampler: Node<ShaderSampler>,
    current: Node<Vec2<f32>>, // 0- 1
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

struct DefaultMipmapReducer;

impl Mipmap2dReducer for DefaultMipmapReducer {
  #[rustfmt::skip]
  fn reduce(
    &self,
    previous_level: Node<ShaderTexture2D>,
    sampler: Node<ShaderSampler>,
    current: Node<Vec2<f32>>, // 0- 1
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let mut r = previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(0., 0.)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(1., 0.)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(0., 1.)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(1., 1.)), consts(0.));
    r
  }
}
struct Mipmap2DGeneratorTask<'a> {
  view: GPU2DTextureView,
  reducer: &'a dyn Mipmap2dReducer,
}

impl<'a> ShaderPassBuilder for Mipmap2DGeneratorTask<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.view, SB::Pass);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Pass);
  }
}

impl<'a> ShaderGraphProvider for Mipmap2DGeneratorTask<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let position = builder.query::<FragmentPosition>()?.xy();
      let texel_size = builder.query::<TexelSize>()?;
      let previous_level = binding.uniform_by(&self.view, SB::Pass);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Pass);

      let result = self
        .reducer
        .reduce(previous_level, sampler, position, texel_size);

      builder.set_fragment_out(0, result)
    })
  }
}
