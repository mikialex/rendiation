use shadergraph::*;

use crate::*;

// https://github.com/BabylonJS/Babylon.js/blob/d25bc29091/packages/dev/core/src/Engines/WebGPU/webgpuTextureHelper.ts

/// Mipmap generation is not supported in webgpu api for now, at least in mvp as far as i known.
/// It's also useful to provide customizable reducer / gen method for proper usage.
///
pub struct Mipmap2DGenerator {
  pub reducer: Box<dyn Mipmap2dReducer>,
}

impl Mipmap2DGenerator {
  pub fn generate(&self, encoder: &mut GPUCommandEncoder, texture: &GPU2DTexture) {
    for level in 0..texture.desc.mip_level_count {
      let mut desc = RenderPassDescriptorOwned::default();

      let view = texture
        .create_view(gpu::TextureViewDescriptor {
          base_mip_level: level,
          mip_level_count: Some(NonZeroU32::new(1).unwrap()),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      desc.channels.push((
        gpu::Operations {
          load: gpu::LoadOp::Load,
          store: true,
        },
        RenderTargetView::Texture(view),
      ));

      let pass = encoder.begin_render_pass(desc);
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
    target: Node<Vec2<u32>>,
  ) -> Node<Vec4<f32>>;
}

struct DefaultMipmapReducer;

impl Mipmap2dReducer for DefaultMipmapReducer {
  fn reduce(
    &self,
    previous_level: Node<ShaderTexture2D>,
    target: Node<Vec2<u32>>,
  ) -> Node<Vec4<f32>> {
    todo!()
  }
}
