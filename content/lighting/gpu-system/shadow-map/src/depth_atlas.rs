use crate::*;

#[derive(Clone)]
pub struct PCFShadowMapGPUData {
  pub atlas: Option<ShadowAtlas>,
  pub pcf_config_parameter: UniformBufferDataView<PCFConfigParameter>,
  pub pcf_config: ShadowPCFConfig,
  pub reversed_depth: bool,
}

impl AbstractShadowMapGPUData for PCFShadowMapGPUData {
  fn check_rebuild(&mut self, required_size: SizeWithDepth, gpu: &GPU) {
    let mut need_rebuild = self.atlas.is_none();
    if let Some(atlas) = &mut self.atlas {
      if atlas.size() != required_size.into_gpu_size() {
        need_rebuild = true;
      }
    }

    if need_rebuild {
      self.atlas = Some(ShadowAtlas::new(
        "basic-shadow-map-atlas",
        required_size.into_gpu_size(),
        gpu,
      ));
    }
  }

  fn clear_shadow_map(&self, frame_ctx: &mut FrameCtx) {
    clear_shadow_map(
      &self.atlas.as_ref().expect("missing check_rebuild"),
      frame_ctx,
      self.reversed_depth,
    );
  }

  fn update_shadow_map(
    &mut self,
    frame_ctx: &mut FrameCtx,
    request: ShadowMapUpdateRequest,
    scene_content: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
  ) {
    let atlas = self.atlas.as_ref().expect("missing check_rebuild");
    let write_view = atlas
      .get_layer_view(request.address.layer_index as u32)
      .clone();

    // custom dispatcher is not required because we only have depth output.
    let pass = pass("shadow-map").with_depth(
      &RenderTargetView::from_texture_view(write_view),
      load_and_store(),
      load_and_store(),
    );

    scene_content(
      frame_ctx,
      ShadowMapDrawRequest {
        shadow_camera_proj: request.shadow_camera_proj.render_matrix,
        shadow_camera_world: request.shadow_camera_world,
        light_id: request.light_id,
        map_desc: ShadowPassDesc {
          desc: pass,
          address: request.address,
        },
      },
    );
  }

  fn create_abstract_shadow_computer(&self) -> Arc<dyn AbstractShadowComputer> {
    Arc::new(PCFComputer {
      shadow_map_atlas: self
        .atlas
        .as_ref()
        .expect("missing check_rebuild")
        .get_full_view()
        .clone(),
      pcf_config_parameter: self.pcf_config_parameter.clone(),
      pcf_config: self.pcf_config,
      reversed_depth: self.reversed_depth,
    })
  }
}

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
