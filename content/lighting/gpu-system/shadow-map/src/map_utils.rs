use crate::*;

pub fn clear_shadow_map(
  map: &GPU2DArrayDepthTextureView,

  frame_ctx: &mut FrameCtx,
  reversed_depth: bool,
) {
  for layer in 0..map.resource.depth_or_array_layers() {
    // clear all
    let write_view = map.resource.create_view(TextureViewDescriptor {
      label: Some("shadowmap-clear-view"),
      dimension: Some(TextureViewDimension::D2),
      base_array_layer: layer,
      array_layer_count: Some(1),
      ..Default::default()
    });

    let _ = pass("shadow-map-clear")
      .with_depth(
        &RenderTargetView::Texture(write_view),
        clear_and_store(if reversed_depth { 0. } else { 1. }),
      )
      .render_ctx(frame_ctx);
  }
}

pub fn get_or_create_shadow_atlas(
  debug_label: &'static str,
  required_size: SizeWithDepth,
  cache: &mut Option<GPU2DArrayDepthTextureView>,
  gpu: &GPU,
) -> GPU2DArrayDepthTextureView {
  let required_size_gpu = required_size.into_gpu_size();
  if let Some(tex) = cache {
    if required_size_gpu != tex.resource.desc.size {
      *cache = None;
    }
  }

  cache
    .get_or_insert_with(|| {
      GPUTexture::create(
        TextureDescriptor {
          label: debug_label.into(),
          size: required_size_gpu,
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
      .unwrap()
    })
    .clone()
}

pub fn sample_shadow_pcf_x36_by_offset(
  map: BindingNode<ShaderDepthTexture2DArray>,
  shadow_position: Node<Vec3<f32>>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  info: ENode<ShadowMapAddressInfo>,
) -> Node<f32> {
  let uv = shadow_position.xy();
  let depth = shadow_position.z();
  let layer = info.layer_index;
  let mut ratio = val(0.0);

  let map_size = map.texture_dimension_2d(None).into_f32();
  let extra_scale = info.size / map_size;

  let uv = uv * extra_scale + info.offset / map_size;

  let s = 2_i32; // we should write a for here?

  for i in -1..=1 {
    for j in -1..=1 {
      let result = map
        .build_compare_sample_call(d_sampler, uv, depth)
        .with_offset((s * i, s * j).into())
        .with_array_index(layer)
        .sample();
      ratio += result;
    }
  }

  ratio / val(9.)
}
