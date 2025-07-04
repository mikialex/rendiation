use rendiation_texture_core::*;
pub use rendiation_texture_packer::pack_2d_to_3d::MultiLayerTexturePackerConfig;
use rendiation_texture_packer::pack_2d_to_3d::*;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct TexturePoolTextureMeta {
  pub layer_index: u32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
  pub require_srgb_to_linear_convert: Bool,
}

impl TexturePoolTextureMeta {
  pub fn none() -> Self {
    TexturePoolTextureMeta {
      layer_index: u32::MAX,
      size: Vec2::zero(),
      offset: Vec2::zero(),
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct TextureSamplerShaderInfo {
  pub address_mode_u: u32,
  pub address_mode_v: u32,
  pub address_mode_w: u32,
  pub mag_filter: u32,
  pub min_filter: u32,
  pub mipmap_filter: u32,
}

const CLAMP_TO_EDGE: u32 = 0;
const REPEAT: u32 = 1;
const MIRRORED_REPEAT: u32 = 2;

const LINEAR: u32 = 1;
const NEAREST: u32 = 0;

fn map_address(mode: rendiation_texture_core::AddressMode) -> u32 {
  match mode {
    rendiation_texture_core::AddressMode::ClampToEdge => CLAMP_TO_EDGE,
    rendiation_texture_core::AddressMode::Repeat => REPEAT,
    rendiation_texture_core::AddressMode::MirrorRepeat => MIRRORED_REPEAT,
  }
}

fn map_filter(mode: rendiation_texture_core::FilterMode) -> u32 {
  match mode {
    rendiation_texture_core::FilterMode::Nearest => NEAREST,
    rendiation_texture_core::FilterMode::Linear => LINEAR,
  }
}

impl From<TextureSampler> for TextureSamplerShaderInfo {
  fn from(value: TextureSampler) -> Self {
    TextureSamplerShaderInfo {
      address_mode_u: map_address(value.address_mode_u),
      address_mode_v: map_address(value.address_mode_v),
      address_mode_w: map_address(value.address_mode_w),
      mag_filter: map_filter(value.mag_filter),
      min_filter: map_filter(value.min_filter),
      mipmap_filter: map_filter(value.mipmap_filter),
      ..Zeroable::zeroed()
    }
  }
}

#[derive(Clone, Debug)]
pub struct TexturePool2dSource {
  pub inner: Arc<GPUBufferImage>,
}

impl PartialEq for TexturePool2dSource {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

pub struct TexturePoolSource {
  texture: Option<GPU2DArrayTextureView>,
  tex_input: RQForker<Texture2DHandle, TexturePool2dSource>,
  packing: BoxedDynReactiveQuery<Texture2DHandle, PackResult2dWithDepth>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  format: TextureFormat,
  address: ReactiveStorageBufferContainer<TexturePoolTextureMeta>,
  samplers: ReactiveStorageBufferContainer<TextureSamplerShaderInfo>,
  gpu: GPU,
}

pub struct TexturePoolSourceInit {
  pub init_sampler_count_capacity: u32,
  pub init_texture_count_capacity: u32,
}

impl TexturePoolSource {
  /// the tex input must cover the full linear global scope, so that we can setup the none exist texture info
  ///
  /// the mipmap is partially supported, the main difference is that we only support max level of min(width, height)
  /// but the specs requires max(width, height); This is the limitation of current implementation and could be
  /// solved in the future
  pub fn new(
    gpu: &GPU,
    config: MultiLayerTexturePackerConfig,
    tex_input: BoxedDynReactiveQuery<Texture2DHandle, Option<TexturePool2dSource>>,
    sampler_input: BoxedDynReactiveQuery<SamplerHandle, TextureSampler>,
    format: TextureFormat,
    init: TexturePoolSourceInit,
  ) -> Self {
    let tex_input = tex_input
      .collective_filter_map(move |tex| {
        let tex = tex?;
        if tex.inner.format != format && tex.inner.format.remove_srgb_suffix() != format {
          return None;
        }
        tex.into()
      })
      .into_boxed()
      .into_forker();

    let size = tex_input.clone().collective_map(|tex| tex.inner.size);
    let full_scope = tex_input.clone().collective_map(|_| {});

    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, Box::new(size));
    let packing = packing.into_forker();
    let add_info = packing
      .clone()
      .collective_map(|v| TexturePoolTextureMeta {
        layer_index: v.depth,
        size: v.result.range.size.into_f32().into(),
        offset: Vec2::new(
          v.result.range.origin.x as f32,
          v.result.range.origin.y as f32,
        ),
        ..Default::default()
      })
      .collective_union(full_scope, |(a, b)| {
        b.map(|_| {
          if let Some(a) = a {
            a
          } else {
            TexturePoolTextureMeta::none()
          }
        })
      })
      .into_query_update_storage(0);

    let srgb_convert_info = tex_input
      .clone()
      .collective_map(|v| Bool::from(v.inner.format.is_srgb()))
      .into_query_update_storage(std::mem::offset_of!(
        TexturePoolTextureMeta,
        require_srgb_to_linear_convert,
      ));

    let address =
      create_reactive_storage_buffer_container(init.init_texture_count_capacity, u32::MAX, gpu)
        .with_source(add_info)
        .with_source(srgb_convert_info);

    let samplers = sampler_input
      .collective_map(TextureSamplerShaderInfo::from)
      .into_query_update_storage(0);

    let samplers =
      create_reactive_storage_buffer_container(init.init_sampler_count_capacity, u32::MAX, gpu)
        .with_source(samplers);

    Self {
      tex_input,
      packing: packing.clone().into_boxed(),
      atlas_resize: Box::new(atlas_resize),
      address,
      samplers,
      texture: None,
      format,
      gpu: gpu.clone(),
    }
  }
}

impl ReactiveGeneralQuery for TexturePoolSource {
  type Output = Box<dyn DynAbstractGPUTextureSystem>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (packing_change, current_pack) = self.packing.describe(cx).resolve_kept();

    if let Poll::Ready(Some(rsize)) = self.atlas_resize.poll_next_unpin(cx) {
      let size = rsize.into_gpu_size();
      self.texture = Some(
        GPUTexture::create(
          TextureDescriptor {
            label: "texture-pool".into(),
            size,
            mip_level_count: MipLevelCount::BySize.get_level_count_wgpu(rsize.size),
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            view_formats: &[],
            usage: TextureUsages::COPY_DST
              | TextureUsages::TEXTURE_BINDING
              | TextureUsages::RENDER_ATTACHMENT,
          },
          &self.gpu.device,
        )
        .create_view(TextureViewDescriptor {
          label: "texture pool view".into(),
          dimension: TextureViewDimension::D2Array.into(),
          ..Default::default()
        })
        .try_into()
        .unwrap(),
      );
    }
    let target = self.texture.as_ref().unwrap();

    let mut encoder = self.gpu.device.create_encoder();

    let (tex_source_change, tex_input_current) = self.tex_input.describe(cx).resolve_kept();
    for (id, change) in tex_source_change.iter_key_value() {
      match change {
        ValueChange::Delta(new_tex, _) => {
          if let Some(current_pack) = current_pack.access(&id) {
            // pack may failed, in this case we do nothing
            let tex = create_gpu_texture2d_with_mipmap(&self.gpu, &mut encoder, &new_tex.inner);
            copy_tex(&mut encoder, &tex, &target.resource, &current_pack);
          }
        }
        ValueChange::Remove(_) => {}
      }
    }

    for (id, change) in packing_change.iter_key_value() {
      match change {
        ValueChange::Delta(new_pack, _) => {
          let mut tex_has_recreated = false;
          if let Some(tex_change) = tex_source_change.access(&id) {
            if !tex_change.is_removed() {
              tex_has_recreated = true;
            }
          }

          // if texture has already created as new texture, we skip the move operation
          if !tex_has_recreated {
            if let Some(tex) = tex_input_current.access(&id) {
              // tex maybe removed
              let tex = create_gpu_texture2d_with_mipmap(&self.gpu, &mut encoder, &tex.inner);
              copy_tex(&mut encoder, &tex, &target.resource, &new_pack);
            }
          }
        }
        ValueChange::Remove(_) => {}
      }
    }

    self.gpu.queue.submit_encoder(encoder);

    self.address.poll_update(cx);
    self.samplers.poll_update(cx);

    Box::new(TexturePool {
      texture: target.clone(),
      address: self.address.target.gpu().clone(),
      samplers: self.samplers.target.gpu().clone(),
    })
  }
}

fn copy_tex(
  encoder: &mut CommandEncoder,
  src: &GPU2DTextureView,
  target: &GPUTexture,
  pack: &PackResult2dWithDepth,
) {
  // note, here we use smaller size, which is different from spec, but simplify the implementation
  let smaller_length = src
    .resource
    .desc
    .size
    .width
    .min(src.resource.desc.size.height);
  let max_mipmap_level = 32 - smaller_length.leading_zeros();

  for mip_level in 0..max_mipmap_level {
    copy_tex_level(encoder, src, target, pack, mip_level);
  }
}

fn copy_tex_level(
  encoder: &mut CommandEncoder,
  src: &GPU2DTextureView,
  target: &GPUTexture,
  pack: &PackResult2dWithDepth,
  mip_level: u32,
) {
  let source = TexelCopyTextureInfo {
    texture: src.resource.gpu_resource(),
    mip_level,
    origin: Origin3d::ZERO,
    aspect: TextureAspect::All,
  };

  let dst = TexelCopyTextureInfo {
    texture: target.gpu_resource(),
    mip_level,
    origin: Origin3d {
      x: pack.result.range.origin.x as u32 >> mip_level,
      y: pack.result.range.origin.y as u32 >> mip_level,
      z: pack.depth,
    },
    aspect: TextureAspect::All,
  };

  let width = src.resource.desc.size.width >> mip_level;
  let height = src.resource.desc.size.height >> mip_level;

  let copy_size = Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };

  encoder.copy_texture_to_texture(source, dst, copy_size);
}

#[derive(Clone)]
pub struct TexturePool {
  texture: GPU2DArrayTextureView,
  address: StorageBufferReadonlyDataView<[TexturePoolTextureMeta]>,
  samplers: StorageBufferReadonlyDataView<[TextureSamplerShaderInfo]>,
}

both!(TexturePoolInShader, ShaderBinding<ShaderTexture2DArray>);
pub struct TexturePoolTextureMetaInShader(pub ShaderReadonlyPtrOf<[TexturePoolTextureMeta]>);
pub struct SamplerPoolInShader(pub ShaderReadonlyPtrOf<[TextureSamplerShaderInfo]>);

impl AbstractIndirectGPUTextureSystem for TexturePool {
  fn bind_system_self(&self, collector: &mut BindingBuilder) {
    collector.bind(&self.texture);
    collector.bind(&self.address);
    collector.bind(&self.samplers);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder
      .bind_by_and_prepare(&self.texture)
      .using_graphics_pair(|r, textures| {
        r.register_typed_both_stage::<TexturePoolInShader>(textures);
      });
    builder
      .bind_by_and_prepare(&self.address)
      .using_graphics_pair(|r, address| {
        r.any_map.register(TexturePoolTextureMetaInShader(address));
      });
    builder
      .bind_by_and_prepare(&self.samplers)
      .using_graphics_pair(|r, samplers| {
        r.any_map.register(SamplerPoolInShader(samplers));
      });
  }
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) {
    let pool = builder.bind_by(&self.texture);
    reg.register_typed_both_stage::<TexturePoolInShader>(pool);
    let address = builder.bind_by(&self.address);
    reg
      .any_map
      .register(TexturePoolTextureMetaInShader(address));
    let samplers = builder.bind_by(&self.samplers);
    reg.any_map.register(SamplerPoolInShader(samplers));
  }

  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = reg
      .try_query_typed_both_stage::<TexturePoolInShader>()
      .unwrap();
    let textures_meta = reg.any_map.get::<TexturePoolTextureMetaInShader>().unwrap();

    let samplers = reg.any_map.get::<SamplerPoolInShader>().unwrap();

    let texture_meta = textures_meta.0.index(shader_texture_handle).load();

    let tex = TexturePoolTextureMeta::layer_index(texture_meta)
      .equals(u32::MAX) // check if the texture is failed to allocate
      .select_branched(
        || val(Vec4::zero()),
        || {
          let sampler = samplers.0.index(shader_sampler_handle).load();
          texture_pool_sample_impl_fn(texture, sampler, texture_meta, uv)
        },
      );

    TexturePoolTextureMeta::require_srgb_to_linear_convert(texture_meta)
      .into_bool()
      .select_branched(
        || {
          let a = tex.w();
          let rgb = tex.xyz();
          let linear = shader_srgb_to_linear_convert_fn(rgb);
          (linear, a).into()
        },
        || tex,
      )
  }
}

// todo, the implementation is not optimal
#[shader_fn]
fn texture_pool_sample_impl(
  texture: BindingNode<ShaderTexture2DArray>,
  sampler: Node<TextureSamplerShaderInfo>,
  texture_meta: Node<TexturePoolTextureMeta>,
  uv: Node<Vec2<f32>>,
) -> Node<Vec4<f32>> {
  let texture_meta = texture_meta.expand();
  let sampler = sampler.expand();

  // todo, we should correct uv after the bilinear offset
  let correct_u = shader_address_mode_fn(sampler.address_mode_u, uv.x());
  let correct_v = shader_address_mode_fn(sampler.address_mode_v, uv.y());
  let uv: Node<Vec2<_>> = (correct_u, correct_v).into();

  let load_position = texture_meta.offset + texture_meta.size * uv;
  let max_load_position = texture_meta.offset + (texture_meta.size - val(Vec2::one()));

  let base_sample_level = if get_current_stage() == Some(ShaderStage::Fragment) {
    let base_sample_level = calculate_mip_level_fn(uv, texture_meta.size);
    let max_mip_level = texture_meta.size.x().min(texture_meta.size.y()).log2();
    base_sample_level.min(max_mip_level)
  } else {
    // force disable mipmap compute(because using dpdx stuff is not supported in none fragment stage)
    val(0.)
  };

  let use_mag_filter = base_sample_level.less_equal_than(val(0.));
  let use_min_filter = use_mag_filter.not();

  let should_use_linear = use_mag_filter
    .and(sampler.mag_filter.equals(LINEAR))
    .or(use_min_filter.and(sampler.min_filter.equals(LINEAR)));

  let base_sample_level_filter_result = sample_texture_level_impl_fn(
    texture,
    load_position,
    max_load_position,
    base_sample_level.into_u32(),
    texture_meta.layer_index,
    should_use_linear,
  );

  let next_sample_level = base_sample_level.ceil();

  let use_mag_filter = next_sample_level.less_equal_than(val(0.));
  let use_min_filter = use_mag_filter.not();

  let should_use_linear = use_mag_filter
    .and(sampler.mag_filter.equals(LINEAR))
    .or(use_min_filter.and(sampler.min_filter.equals(LINEAR)));

  let next_sample_level_filter_result = sample_texture_level_impl_fn(
    texture,
    load_position,
    max_load_position,
    next_sample_level.into_u32(),
    texture_meta.layer_index,
    should_use_linear,
  );

  sampler
    .mipmap_filter
    .equals(NEAREST)
    .select(val(1.), base_sample_level.fract())
    .mix(
      base_sample_level_filter_result,
      next_sample_level_filter_result,
    )
}

#[shader_fn]
fn sample_texture_level_impl(
  texture: BindingNode<ShaderTexture2DArray>,
  raw_load_position: Node<Vec2<f32>>, // in atlas coord space
  max_load_position: Node<Vec2<f32>>, // in atlas coord space
  level: Node<u32>,
  layer: Node<u32>,
  linear: Node<bool>,
) -> Node<Vec4<f32>> {
  linear.select_branched(
    || {
      let xy_mix = raw_load_position.fract();
      let raw_load_position = raw_load_position.floor();

      let p00 = raw_load_position;
      let p10 = raw_load_position + val(Vec2::new(1.0, 0.0));
      let p01 = raw_load_position + val(Vec2::new(0.0, 1.0));
      let p11 = raw_load_position + val(Vec2::new(1.0, 1.0));

      let p00 = sample_texture_impl(texture, p00.min(max_load_position), level, layer);
      let p10 = sample_texture_impl(texture, p10.min(max_load_position), level, layer);
      let p01 = sample_texture_impl(texture, p01.min(max_load_position), level, layer);
      let p11 = sample_texture_impl(texture, p11.min(max_load_position), level, layer);

      let p0 = xy_mix.x().mix(p00, p10);
      let p1 = xy_mix.x().mix(p01, p11);
      xy_mix.y().mix(p0, p1)
    },
    || sample_texture_impl(texture, raw_load_position, layer, level),
  )
}

fn sample_texture_impl(
  texture: BindingNode<ShaderTexture2DArray>,
  load_position_f32: Node<Vec2<f32>>,
  level: Node<u32>,
  layer: Node<u32>,
) -> Node<Vec4<f32>> {
  let x = load_position_f32.x().floor().into_u32();
  let y = load_position_f32.y().floor().into_u32();
  texture.load_texel_layer((x >> level, y >> level).into(), layer, level)
}

#[shader_fn]
fn shader_srgb_to_linear_convert(srgb: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  (
    shader_srgb_to_linear_convert_per_channel_fn(srgb.x()),
    shader_srgb_to_linear_convert_per_channel_fn(srgb.y()),
    shader_srgb_to_linear_convert_per_channel_fn(srgb.z()),
  )
    .into()
}

#[shader_fn]
fn shader_srgb_to_linear_convert_per_channel(c: Node<f32>) -> Node<f32> {
  c.less_than(0.04045).select_branched(
    || c * val(0.0773993808),
    || (c * val(0.9478672986) + val(0.0521327014)).pow(2.4),
  )
}

#[shader_fn]
fn shader_address_mode(mode: Node<u32>, uv: Node<f32>) -> Node<f32> {
  let result = uv.make_local_var();
  switch_by(mode)
    .case(CLAMP_TO_EDGE, || result.store(uv.max(0.0).min(1.0)))
    .case(REPEAT, || result.store(uv - uv.floor()))
    .case(MIRRORED_REPEAT, || {
      let is_even = (uv.floor().into_i32() % val(2)).equals(0);
      let uv = is_even.select(uv, -uv);
      result.store(uv - uv.floor())
    })
    .end_with_default(|| {});
  result.load()
}

#[shader_fn]
fn calculate_mip_level_impl(uv: Node<Vec2<f32>>) -> Node<f32> {
  let dx = uv.dpdx();
  let dy = uv.dpdy();
  // (dx.dot(dx) + dy.dot(dy)).sqrt().log2().floor()
  let delta_max_sqr = dx.dot(dx).max(dy.dot(dy));
  val(0.5) * delta_max_sqr.log2()
}

// https://bgolus.medium.com/distinctive-derivative-differences-cce38d36797b
#[shader_fn]
fn calculate_mip_level(uv: Node<Vec2<f32>>, size: Node<Vec2<f32>>) -> Node<f32> {
  let uv2: Node<Vec2<f32>> = ((uv.x() - val(0.5)).fract(), uv.y()).into();
  let a = calculate_mip_level_impl(uv2 * size);
  let b = calculate_mip_level_impl(uv * size);
  a.min(b)
}
