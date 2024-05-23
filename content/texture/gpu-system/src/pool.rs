use rendiation_texture_core::*;
use rendiation_texture_packer::pack_2d_to_3d::*;

use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
struct TextureAddressInfo {
  pub layer_index: u32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
struct TextureSamplerShaderInfo {
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

fn map_address(mode: rendiation_texture_core::AddressMode) -> u32 {
  match mode {
    rendiation_texture_core::AddressMode::ClampToEdge => CLAMP_TO_EDGE,
    rendiation_texture_core::AddressMode::Repeat => REPEAT,
    rendiation_texture_core::AddressMode::MirrorRepeat => MIRRORED_REPEAT,
  }
}

impl From<TextureSampler> for TextureSamplerShaderInfo {
  fn from(value: TextureSampler) -> Self {
    TextureSamplerShaderInfo {
      address_mode_u: map_address(value.address_mode_u),
      address_mode_v: map_address(value.address_mode_v),
      address_mode_w: map_address(value.address_mode_w),
      mag_filter: 0,
      min_filter: 0,
      mipmap_filter: 0,
      ..Zeroable::zeroed()
    }
  }
}

pub struct TexturePoolSource {
  texture: GPUTexture,
  address: StorageBufferReadOnlyDataView<[TextureAddressInfo]>,
  samplers: StorageBufferReadOnlyDataView<[TextureSamplerShaderInfo]>,
  tex_input: Box<dyn ReactiveCollection<Texture2DHandle, GPU2DTextureView>>,
  sampler_input: Box<dyn ReactiveCollection<SamplerHandle, TextureSampler>>,
  packing: Box<dyn ReactiveCollection<Texture2DHandle, PackResult2dWithDepth>>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
}

impl TexturePoolSource {
  pub fn new(
    gpu: &GPUResourceCtx,
    config: MultiLayerTexturePackerConfig,
    tex_input: Box<dyn ReactiveCollection<Texture2DHandle, GPU2DTextureView>>,
    sampler_input: Box<dyn ReactiveCollection<SamplerHandle, TextureSampler>>,
    format: TextureFormat,
  ) -> (
    Self,
    Box<dyn ReactiveCollection<Texture2DHandle, PackResult2dWithDepth>>,
  ) {
    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, todo!());
    let packing = packing.into_forker();

    let texture = GPUTexture::create(
      TextureDescriptor {
        label: "texture-pool".into(),
        size: config.init_size.into_gpu_size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        view_formats: &[],
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    );

    let sys = Self {
      tex_input,
      sampler_input,
      packing: packing.clone().into_boxed(),
      atlas_resize: Box::new(atlas_resize),
      address: todo!(),
      samplers: todo!(),
      texture,
    };
    (sys, packing.into_boxed())
  }
}

impl ReactiveState for TexturePoolSource {
  type State = TexturePool;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    TexturePool {
      texture: todo!(),
      address: todo!(),
      samplers: todo!(),
    }
  }
}

pub struct TexturePool {
  texture: GPU2DArrayTextureView,
  address: StorageBufferReadOnlyDataView<[TextureAddressInfo]>,
  samplers: StorageBufferReadOnlyDataView<[TextureSamplerShaderInfo]>,
}

both!(TexturePoolInShader, ShaderHandlePtr<ShaderTexture2DArray>);
both!(
  TexturePoolAddressInfoInShader,
  ShaderReadOnlyStoragePtr<[TextureAddressInfo]>
);
both!(
  SamplerPoolInShader,
  ShaderReadOnlyStoragePtr<[TextureSamplerShaderInfo]>
);

impl AbstractIndirectGPUTextureSystem for TexturePool {
  fn bind_system_self(&self, collector: &mut BindingBuilder) {
    collector.bind(&self.texture);
    collector.bind(&self.address);
    collector.bind(&self.samplers);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder
      .bind_by(&self.texture)
      .using_graphics_pair(builder, |r, textures| {
        r.register_typed_both_stage::<TexturePoolInShader>(textures);
      });
    builder
      .bind_by(&self.address)
      .using_graphics_pair(builder, |r, address| {
        r.register_typed_both_stage::<TexturePoolAddressInfoInShader>(address);
      });
    builder
      .bind_by(&self.samplers)
      .using_graphics_pair(builder, |r, samplers| {
        r.register_typed_both_stage::<SamplerPoolInShader>(samplers);
      });
  }

  /// todo, mipmap
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = reg.query_typed_both_stage::<TexturePoolInShader>().unwrap();
    let address = reg
      .query_typed_both_stage::<TexturePoolAddressInfoInShader>()
      .unwrap();

    let samplers = reg.query_typed_both_stage::<SamplerPoolInShader>().unwrap();

    let texture_address = address.index(shader_texture_handle).load().expand();
    let sampler = samplers.index(shader_sampler_handle).load().expand();

    let correct_u = shader_address_mode(sampler.address_mode_u, uv.x());
    let correct_v = shader_address_mode(sampler.address_mode_v, uv.y());
    let uv: Node<Vec2<_>> = (correct_u, correct_v).into();

    let load_position = texture_address.offset + texture_address.size * uv;
    let load_position_x = load_position.x().floor().into_u32();
    let load_position_y = load_position.y().floor().into_u32();

    texture.load_texel_layer(
      (load_position_x, load_position_y).into(),
      texture_address.layer_index,
      val(0),
    )
  }
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
