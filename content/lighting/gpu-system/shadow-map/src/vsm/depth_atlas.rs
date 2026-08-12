use super::*;
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VSMConfig {
  /// the blur kernel size in texels of the vsm pre-filter passes
  pub filter_size: f32,
  /// the variance bias, scaled by 0.01 to get the min variance of the
  /// chebyshev upper bound
  pub vsm_bias: f32,
  pub light_bleeding_reduction: f32,
}

impl Default for VSMConfig {
  fn default() -> Self {
    Self {
      filter_size: 3.,
      vsm_bias: 0.,
      light_bleeding_reduction: 0.,
    }
  }
}

/// See [VSMConfig]
#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Debug, PartialEq, ShaderStruct)]
pub struct VSMConfigUniform {
  pub filter_size: f32,
  pub vsm_bias: f32,
  pub light_bleeding_reduction: f32,
}

#[derive(Clone)]
pub struct VSMShadowMap {
  depth_map: Option<ShadowAtlas>,
  vsm_map: Option<VsmMapGenerator>,
  config: VSMConfig,
  config_uniform: UniformBufferDataView<VSMConfigUniform>,
  reversed_depth: bool,
}

impl VSMShadowMap {
  pub fn new(config: VSMConfig, reversed_depth: bool, gpu: &GPU) -> Self {
    Self {
      depth_map: None,
      vsm_map: None,
      config_uniform: Self::create_config_uniform(config, gpu),
      config,
      reversed_depth,
    }
  }

  fn create_config_uniform(
    config: VSMConfig,
    gpu: &GPU,
  ) -> UniformBufferDataView<VSMConfigUniform> {
    create_uniform(
      VSMConfigUniform {
        filter_size: config.filter_size,
        vsm_bias: config.vsm_bias,
        light_bleeding_reduction: config.light_bleeding_reduction,
        ..Zeroable::zeroed()
      },
      &gpu.device,
      "vsm-config",
    )
  }

  /// update the config, the uniform is recreated because the buffer content
  /// is only visible after the next submit, todo, cache this using hooks
  pub fn update_config(&mut self, config: VSMConfig, gpu: &GPU) {
    self.config = config;
    self.config_uniform = Self::create_config_uniform(config, gpu);
  }
}

impl AbstractShadowMapGPUData for VSMShadowMap {
  fn check_rebuild(&mut self, required_size: SizeWithDepth, gpu: &GPU) {
    let required_size = required_size.into_gpu_size();
    let mut need_rebuild = self.depth_map.is_none() || self.vsm_map.is_none();
    if let Some(atlas) = &self.depth_map
      && atlas.size() != required_size
    {
      need_rebuild = true;
    }
    if let Some(generator) = &self.vsm_map
      && generator.atlas.size() != required_size
    {
      need_rebuild = true;
    }
    if need_rebuild {
      self.depth_map = Some(ShadowAtlas::new("vsm-depth-atlas", required_size, gpu));
      self.vsm_map = Some(VsmMapGenerator::new("vsm-map-atlas", required_size, gpu));
    }
  }

  fn clear_shadow_map(&self, frame_ctx: &mut FrameCtx) {
    clear_shadow_map(
      self.depth_map.as_ref().expect("missing check_rebuild"),
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
    let depth_map = self.depth_map.as_ref().expect("missing check_rebuild");
    let write_view = depth_map
      .get_layer_view(request.address.layer_index as u32)
      .clone();
    let depth_view = depth_map.get_full_view().clone();

    // render the shadow depth of the light region into the depth atlas
    let pass = pass("vsm-shadow-map-depth").with_depth(
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

    // convert the rendered depth into the vsm moments and blur them
    let generator = self.vsm_map.as_mut().expect("missing check_rebuild");
    generator.update_light(
      frame_ctx,
      &depth_view,
      extract_shadow_proj_linear_depth_recover_helper(request.shadow_camera_proj.opengl_ndc_matrix),
      self.reversed_depth,
      request.address,
      self.config.filter_size,
    );
  }

  fn create_abstract_shadow_computer(&self) -> Arc<dyn AbstractShadowComputer> {
    Arc::new(VSMComputer {
      vsm_map_atlas: self
        .vsm_map
        .as_ref()
        .expect("missing check_rebuild")
        .atlas
        .get_full_view()
        .clone(),
      config: self.config_uniform.clone(),
      reversed_depth: self.reversed_depth,
    })
  }
}

/// the maximum blur kernel size of the vsm blur passes, in texels
pub const MAX_VSM_FILTER_SIZE: f32 = 9.0;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct VsmMapProcessor {
  pub proj_linear_depth_recover_helper: ProjLinearDepthRecoverHelper,
  /// the blur kernel size in texels, clamped to [1, MAX_VSM_FILTER_SIZE]
  pub filter_size: f32,
}

/// the moments atlas of the variance shadow maps, each layer stores the two
/// moments (depth, depth * depth) of the shadow map regions, the depth is
/// the linear depth in [0, 1], the temp view is the intermediate target of
/// the separable blur passes
#[derive(Clone)]
pub struct VsmAtlas {
  texture: GPU2DArrayTextureView,
  view_for_each_layer: Arc<Vec<GPU2DTextureView>>,
  /// the intermediate target of the separable blur passes
  /// todo, allocate the temp view as the largest light region size instead
  /// of the whole atlas size, this requires remapping the region coordinate
  /// to the temp space in the blur passes
  temp_view: GPU2DTextureView,
}

impl VsmAtlas {
  pub fn new(debug_label: &'static str, size: Extent3d, gpu: &GPU) -> Self {
    let texture: GPU2DArrayTextureView = GPUTexture::create(
      TextureDescriptor {
        label: debug_label.into(),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rg32Float,
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
          label: Some("vsm-layer-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: layer,
          array_layer_count: Some(1),
          ..Default::default()
        })
      })
      .map(|view| view.try_into().unwrap())
      .collect::<Vec<_>>();

    let temp_view: GPU2DTextureView = GPUTexture::create(
      TextureDescriptor {
        label: debug_label.into(),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rg32Float,
        view_formats: &[],
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    )
    .create_view(TextureViewDescriptor {
      dimension: Some(TextureViewDimension::D2),
      ..Default::default()
    })
    .try_into()
    .unwrap();

    Self {
      texture,
      view_for_each_layer: Arc::new(view_for_each_layer),
      temp_view,
    }
  }

  pub fn get_layer_view(&self, layer: u32) -> &GPU2DTextureView {
    &self.view_for_each_layer[layer as usize]
  }

  pub fn get_full_view(&self) -> &GPU2DArrayTextureView {
    &self.texture
  }

  pub fn get_temp_view(&self) -> &GPU2DTextureView {
    &self.temp_view
  }

  pub fn size(&self) -> Extent3d {
    self.texture.resource.desc.size
  }
}

/// generates the variance shadow maps from the rendered depth atlas,
/// for each light region the convert pass and the two separable blur
/// passes are executed, the generator owns the moments atlas
#[derive(Clone)]
pub struct VsmMapGenerator {
  pub atlas: VsmAtlas,
  horizontal_direction: UniformBufferDataView<Vec4<f32>>,
  vertical_direction: UniformBufferDataView<Vec4<f32>>,
}

impl VsmMapGenerator {
  pub fn new(debug_label: &'static str, size: Extent3d, gpu: &GPU) -> Self {
    let horizontal_direction =
      create_uniform(Vec4::new(1., 0., 0., 0.), &gpu.device, "vsm-blur-direction");
    let vertical_direction =
      create_uniform(Vec4::new(0., 1., 0., 0.), &gpu.device, "vsm-blur-direction");
    Self {
      atlas: VsmAtlas::new(debug_label, size, gpu),
      horizontal_direction,
      vertical_direction,
    }
  }

  /// update the moments of one light region, the depth of the region must
  /// already be rendered into the depth atlas layer, the helper is extracted
  /// from the opengl ndc projection of the shadow camera
  pub fn update_light(
    &mut self,
    ctx: &mut FrameCtx,
    depth_atlas: &GPU2DArrayDepthTextureView,
    proj_linear_depth_recover_helper: ProjLinearDepthRecoverHelper,
    reversed_depth: bool,
    address: ShadowMapAddressInfo,
    filter_size: f32,
  ) {
    let layer = address.layer_index as u32;
    let x = address.offset.x;
    let y = address.offset.y;
    let w = address.size.x;
    let h = address.size.y;
    let layer_view = self.atlas.get_layer_view(layer).texture.clone();
    let temp_view = self.atlas.get_temp_view().texture.clone();

    let base_config = VsmMapProcessor {
      proj_linear_depth_recover_helper,
      filter_size,
      ..Default::default()
    };

    // the uniforms are created per light instead of reused with a delayed
    // write, because the buffer content is only visible after the next
    // submit and all lights would read the last written value otherwise
    // todo, cache this using hooks
    let config = create_uniform(base_config, &ctx.gpu.device, "vsm-map-config");
    let map_info = create_uniform(address, &ctx.gpu.device, "vsm-map-info");

    // the convert pass, writes the linear depth moments into the vsm atlas layer
    let convert_task = VsmConvertTask {
      input: depth_atlas,
      config: &config,
      map_info: &map_info,
      reversed_depth,
    };
    let mut convert = pass("vsm-convert")
      .with_color(
        &RenderTargetView::from_texture_view(layer_view.clone()),
        load_and_store(),
      )
      .render_ctx(ctx);
    convert.pass.ctx.pass.set_viewport(x, y, w, h, 0., 1.);
    convert.by(&mut convert_task.draw_quad());

    // the horizontal blur pass, reads the vsm atlas layer and writes the temp view
    let blur_h_task = VsmBlurTask {
      input: self.atlas.get_layer_view(layer),
      config: &config,
      direction: &self.horizontal_direction,
      map_info: &map_info,
    };
    let mut blur_h = pass("vsm-blur-h")
      .with_color(
        &RenderTargetView::from_texture_view(temp_view.clone()),
        load_and_store(),
      )
      .render_ctx(ctx);
    blur_h.pass.ctx.pass.set_viewport(x, y, w, h, 0., 1.);
    blur_h.by(&mut blur_h_task.draw_quad());

    // the vertical blur pass, reads the temp view and writes back the vsm atlas layer
    let blur_v_task = VsmBlurTask {
      input: self.atlas.get_temp_view(),
      config: &config,
      direction: &self.vertical_direction,
      map_info: &map_info,
    };
    let mut blur_v = pass("vsm-blur-v")
      .with_color(
        &RenderTargetView::from_texture_view(layer_view),
        load_and_store(),
      )
      .render_ctx(ctx);
    blur_v.pass.ctx.pass.set_viewport(x, y, w, h, 0., 1.);
    blur_v.by(&mut blur_v_task.draw_quad());
  }
}
