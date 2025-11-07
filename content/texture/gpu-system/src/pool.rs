use std::borrow::Cow;

use fast_hash_collection::FastHashSet;
use rendiation_shader_library::color::shader_srgb_to_linear_convert_fn;
use rendiation_texture_core::*;
pub use rendiation_texture_packer::pack_2d_to_3d::MultiLayerTexturePackerConfig;
use rendiation_texture_packer::pack_2d_to_3d::*;

use crate::*;

pub const TEXTURE_POOL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct TexturePoolTextureMeta {
  pub layout: TexturePoolTextureMetaLayoutInfo,
  pub require_srgb_to_linear_convert: Bool,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct TexturePoolTextureMetaLayoutInfo {
  pub layer_index: u32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}

impl From<PackResult2dWithDepth> for TexturePoolTextureMetaLayoutInfo {
  fn from(v: PackResult2dWithDepth) -> Self {
    Self {
      layer_index: v.depth,
      size: v.result.range.size.into_f32().into(),
      offset: Vec2::new(
        v.result.range.origin.x as f32,
        v.result.range.origin.y as f32,
      ),
      ..Default::default()
    }
  }
}

impl From<Option<PackResult2dWithDepth>> for TexturePoolTextureMetaLayoutInfo {
  fn from(v: Option<PackResult2dWithDepth>) -> Self {
    v.map(Self::from).unwrap_or(Self::none())
  }
}

impl TexturePoolTextureMetaLayoutInfo {
  pub fn none() -> Self {
    Self {
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

use serde::*;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug, Copy)]
pub struct TexturePoolSourceInit {
  pub init_sampler_count_capacity: u32,
  pub init_texture_count_capacity: u32,
  pub atlas_config: MultiLayerTexturePackerConfig,
}

pub fn update_atlas(
  gpu: &GPU,
  encoder: &mut GPUCommandEncoder,
  atlas: &mut Option<GPU2DArrayTextureView>,
  format: TextureFormat,
  current_pack: impl Fn(u32) -> Option<PackResult2dWithDepth>,
  packing_change: impl Iterator<Item = (u32, ValueChange<Option<PackResult2dWithDepth>>)>,
  tex_input_current: impl Fn(u32) -> Arc<GPUBufferImage>,
  tex_source_change: impl Iterator<Item = (u32, Arc<GPUBufferImage>)>,
  size_request: SizeWithDepth,
) {
  if let Some(a) = atlas {
    if a.resource.desc.size != size_request.into_gpu_size() {
      *atlas = None;
    }
  }

  let target = atlas.get_or_insert_with(|| {
    GPUTexture::create(
      TextureDescriptor {
        label: "texture-pool".into(),
        size: size_request.into_gpu_size(),
        mip_level_count: MipLevelCount::BySize.get_level_count_wgpu(size_request.size),
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        view_formats: &[],
        usage: TextureUsages::COPY_DST
          | TextureUsages::TEXTURE_BINDING
          | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    )
    .create_view(TextureViewDescriptor {
      label: "texture pool view".into(),
      dimension: TextureViewDimension::D2Array.into(),
      ..Default::default()
    })
    .try_into()
    .unwrap()
  });

  let mut changed_set = FastHashSet::default();

  let should_normalize_srgb = gpu.info().adaptor_info.backend == Backend::Gl;

  for (id, new_tex) in tex_source_change {
    changed_set.insert(id);
    if let Some(current_pack) = current_pack(id) {
      // pack may failed, in this case we do nothing
      if let Some(tex) = normalize_format(&new_tex, should_normalize_srgb) {
        let tex = create_gpu_texture2d_with_mipmap(gpu, encoder, &tex);
        copy_tex(encoder, &tex, &target.resource, &current_pack);
      }
    }
  }

  for (id, change) in packing_change {
    match change {
      ValueChange::Delta(new_pack, _) => {
        let mut tex_has_recreated = false;
        if changed_set.contains(&id) {
          tex_has_recreated = true;
        }

        // if texture has already created as new texture, we skip the move operation
        if !tex_has_recreated {
          let tex = tex_input_current(id);
          // tex maybe removed
          if let Some(new_pack) = new_pack {
            if let Some(tex) = normalize_format(&tex, should_normalize_srgb) {
              let tex = create_gpu_texture2d_with_mipmap(gpu, encoder, &tex);
              copy_tex(encoder, &tex, &target.resource, &new_pack);
            }
          }
        }
      }
      ValueChange::Remove(_) => {}
    }
  }
}

fn srgb_to_linear_convert_per_channel(c: f32) -> f32 {
  if c <= 0.04045 {
    c * 0.0773993808
  } else {
    (c * 0.9478672986 + 0.0521327014).powf(2.4)
  }
}

fn normalize_format(tex: &GPUBufferImage, normalize_srgb: bool) -> Option<Cow<'_, GPUBufferImage>> {
  if tex.format == TEXTURE_POOL_FORMAT {
    return Cow::Borrowed(tex).into();
  }

  if !normalize_srgb && tex.format.remove_srgb_suffix() == TEXTURE_POOL_FORMAT.remove_srgb_suffix()
  {
    return Cow::Borrowed(tex).into();
  }

  log::warn!("texture pool try normalize texture");

  let data = match tex.format {
    TextureFormat::Rgba8UnormSrgb => {
      let data: &[u8] = cast_slice(&tex.data);
      data
        .iter()
        .map(|v| *v as f32 / 255.)
        .map(srgb_to_linear_convert_per_channel)
        .map(|v| (v * 255.) as u8)
        .collect()
    }
    TextureFormat::R8Unorm => {
      let data: &[u8] = cast_slice(&tex.data);
      data.iter().flat_map(|v| [*v, 0, 0, 0]).collect()
    }
    _ => {
      log::warn!(
        "texture pool not support normalize format {:?} to pool format {:?}",
        tex.format,
        TEXTURE_POOL_FORMAT
      );
      return None;
    }
  };

  Cow::<'_, GPUBufferImage>::Owned(GPUBufferImage {
    data,
    format: TEXTURE_POOL_FORMAT,
    size: tex.size,
  })
  .into()
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
  pub texture: GPU2DArrayTextureView,
  pub address: AbstractReadonlyStorageBuffer<[TexturePoolTextureMeta]>,
  pub samplers: AbstractReadonlyStorageBuffer<[TextureSamplerShaderInfo]>,
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
        r.register_typed_both_stage::<TexturePoolInShader>(*textures);
      });
    builder
      .bind_by_and_prepare(&self.address)
      .using_graphics_pair(|r, address| {
        r.any_map
          .register(TexturePoolTextureMetaInShader(address.clone()));
      });
    builder
      .bind_by_and_prepare(&self.samplers)
      .using_graphics_pair(|r, samplers| {
        r.any_map.register(SamplerPoolInShader(samplers.clone()));
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

  fn compute_base_level(
    &self,
    reg: &SemanticRegistry,
    uv: Node<Vec2<f32>>,
    shader_texture_handle: Node<Texture2DHandle>,
    _shader_sampler_handle: Node<SamplerHandle>,
  ) -> Node<f32> {
    if get_current_stage() == Some(ShaderStage::Fragment) {
      let textures_meta = reg.any_map.get::<TexturePoolTextureMetaInShader>().unwrap();
      let texture_meta = textures_meta.0.index(shader_texture_handle);
      let size = texture_meta.layout().size().load();
      calculate_mip_level_fn(uv, size)
    } else {
      // force disable mipmap compute(because using dpdx stuff is not supported in none fragment stage)
      val(0.)
    }
  }

  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
    base_level: Node<f32>,
  ) -> Node<Vec4<f32>> {
    let texture = reg
      .try_query_typed_both_stage::<TexturePoolInShader>()
      .unwrap();
    let textures_meta = reg.any_map.get::<TexturePoolTextureMetaInShader>().unwrap();

    let samplers = reg.any_map.get::<SamplerPoolInShader>().unwrap();

    let texture_meta = textures_meta.0.index(shader_texture_handle).load();
    let texture_layout = TexturePoolTextureMeta::layout(texture_meta);

    let tex = TexturePoolTextureMetaLayoutInfo::layer_index(texture_layout)
      .equals(u32::MAX) // check if the texture is failed to allocate
      .select_branched(
        || val(Vec4::zero()),
        || {
          let sampler = samplers.0.index(shader_sampler_handle).load();
          texture_pool_sample_impl_fn(texture, sampler, texture_layout, uv, base_level)
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
  texture_meta: Node<TexturePoolTextureMetaLayoutInfo>,
  uv: Node<Vec2<f32>>,
  base_sample_level: Node<f32>,
) -> Node<Vec4<f32>> {
  let texture_meta = texture_meta.expand();
  let sampler = sampler.expand();

  // todo, we should correct uv after the bilinear offset
  let correct_u = shader_address_mode_fn(sampler.address_mode_u, uv.x());
  let correct_v = shader_address_mode_fn(sampler.address_mode_v, uv.y());
  let uv: Node<Vec2<_>> = (correct_u, correct_v).into();

  let load_position = texture_meta.offset + texture_meta.size * uv;
  let max_load_position = texture_meta.offset + (texture_meta.size - val(Vec2::one()));

  let max_mip_level = texture_meta.size.x().min(texture_meta.size.y()).log2();
  let base_sample_level = base_sample_level.min(max_mip_level);

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
    || sample_texture_impl(texture, raw_load_position, level, layer),
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
