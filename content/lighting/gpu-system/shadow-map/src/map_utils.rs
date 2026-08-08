use crate::*;

/// this struct mainly use for cache view for each layer
#[derive(Clone)]
pub struct ShadowAtlas {
  texture: GPU2DArrayDepthTextureView,
  view_for_each_layer: Arc<Vec<GPUTextureView>>,
}

impl ShadowAtlas {
  pub fn get_layer_view(&self, layer: u32) -> &GPUTextureView {
    &self.view_for_each_layer[layer as usize]
  }
  pub fn get_full_view(&self) -> &GPU2DArrayDepthTextureView {
    &self.texture
  }
}

impl ShadowAtlas {
  pub fn new(debug_label: &'static str, size: Extent3d, gpu: &GPU) -> Self {
    let texture: GPU2DArrayDepthTextureView = GPUTexture::create(
      TextureDescriptor {
        label: debug_label.into(),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        view_formats: &[],
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    )
    .create_view(TextureViewDescriptor {
      dimension: TextureViewDimension::D2Array.into(),
      ..Default::default()
    })
    .try_into()
    .unwrap();

    let view_for_each_layer = (0..size.depth_or_array_layers)
      .map(|layer| {
        texture.resource.create_view(TextureViewDescriptor {
          label: Some("shadowmap-layer-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: layer,
          array_layer_count: Some(1),
          ..Default::default()
        })
      })
      .collect::<Vec<_>>();

    Self {
      texture,
      view_for_each_layer: Arc::new(view_for_each_layer),
    }
  }

  pub fn size(&self) -> Extent3d {
    self.texture.resource.desc.size
  }
}

// todo, only clear layer that has allocated shadow
pub fn clear_shadow_map(atlas: &ShadowAtlas, frame_ctx: &mut FrameCtx, reversed_depth: bool) {
  let map = &atlas.texture;
  for layer in 0..map.resource.depth_or_array_layers() {
    // clear all
    let write_view = atlas.get_layer_view(layer).clone();

    let _ = pass("shadow-map-clear")
      .with_depth(
        &RenderTargetView::from_texture_view(write_view),
        clear_and_store(if reversed_depth { 0. } else { 1. }),
        load_and_store(),
      )
      .render_ctx(frame_ctx);
  }
}

pub fn get_or_create_shadow_atlas(
  debug_label: &'static str,
  required_size: SizeWithDepth,
  cache: &mut Option<ShadowAtlas>,
  gpu: &GPU,
) -> ShadowAtlas {
  let required_size_gpu = required_size.into_gpu_size();
  if let Some(tex) = cache {
    if required_size_gpu != tex.size() {
      *cache = None;
    }
  }

  cache
    .get_or_insert_with(|| ShadowAtlas::new(debug_label, required_size_gpu, gpu))
    .clone()
}
