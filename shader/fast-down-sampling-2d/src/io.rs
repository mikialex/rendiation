use std::{any::TypeId, hash::Hash};

use crate::*;

pub trait FastDownSamplingIO<V>: ShaderHashProvider {
  fn root_size(&self) -> (u32, u32);
  fn mip_level_count(&self) -> u32;

  fn bind_first_stage_shader(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn FastDownSamplingIOFirstStageInvocation<V>>;
  fn bind_first_stage_pass(&self, cx: &mut BindingBuilder);

  fn bind_second_stage_shader(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn FastDownSamplingIOSecondStageInvocation<V>>;
  fn bind_second_stage_pass(&self, cx: &mut BindingBuilder);
}

pub trait FastDownSamplingIOFirstStageInvocation<V> {
  /// the root level data maybe not exist and computed from other source, this trait support this case
  ///
  /// the other way to solve is to use another pass to writer the root data at the cost of extra bandwidth
  fn get_root_loader_with_possible_write(&self) -> Box<dyn SourceImageLoader<V>>;
  fn get_1_6_level_writer(&self, absolute_index: usize) -> Box<dyn SourceImageWriter<V>>;
}

pub trait FastDownSamplingIOSecondStageInvocation<V> {
  fn get_level_6_loader(&self) -> Box<dyn SourceImageLoader<V>>;
  fn get_7_12_level_writer(&self, absolute_index: usize) -> Box<dyn SourceImageWriter<V>>;
}

pub struct CommonTextureFastDownSamplingSource<F: 'static, V: 'static> {
  pub target: GPUTypedTexture<TextureDimension2, F>,

  pub base: StorageTextureViewReadonly2D<F>,
  pub first_pass_writes: [StorageTextureViewWriteonly2D<F>; 6],
  pub second_pass_read: StorageTextureViewReadonly2D<F>,
  pub second_pass_writes: [StorageTextureViewWriteonly2D<F>; 5],

  pub texel_to_reduce_unit:
    fn(BindingNode<ShaderStorageTextureR2D<F>>) -> Box<dyn SourceImageLoader<V>>,
  pub reduce_unit_to_texel:
    fn(BindingNode<ShaderStorageTextureW2D<F>>) -> Box<dyn SourceImageWriter<V>>,
}

impl<F: TextureFormatDynamicCheck + 'static, V: 'static> CommonTextureFastDownSamplingSource<F, V> {
  pub fn new(
    target: &GPUTypedTexture<TextureDimension2, F>,
    texel_to_reduce_unit: fn(
      BindingNode<ShaderStorageTextureR2D<F>>,
    ) -> Box<dyn SourceImageLoader<V>>,
    reduce_unit_to_texel: fn(
      BindingNode<ShaderStorageTextureW2D<F>>,
    ) -> Box<dyn SourceImageWriter<V>>,
  ) -> Self {
    // level that exceeds will be clamped to max level
    let mipmaps: [GPUTypedTextureView<TextureDimension2, F>; 13] = std::array::from_fn(|index| {
      target
        .create_view(TextureViewDescriptor {
          base_mip_level: (index as u32).clamp(0, target.mip_level_count() - 1),
          mip_level_count: Some(1),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap()
    });

    let base = mipmaps[0]
      .clone()
      .into_storage_texture_view_readonly()
      .unwrap();

    let first_pass_writes = std::array::from_fn(|index| {
      mipmaps[index + 1]
        .clone()
        .into_storage_texture_view_writeonly()
        .unwrap()
    });

    let second_pass_read = mipmaps[6]
      .clone()
      .into_storage_texture_view_readonly()
      .unwrap();

    let second_pass_write = std::array::from_fn(|index| {
      mipmaps[index + 7]
        .clone()
        .into_storage_texture_view_writeonly()
        .unwrap()
    });

    Self {
      target: target.clone(),
      base,
      first_pass_writes,
      second_pass_read,
      second_pass_writes: second_pass_write,
      texel_to_reduce_unit,
      reduce_unit_to_texel,
    }
  }
}

impl<F: 'static, V: 'static> ShaderHashProvider for CommonTextureFastDownSamplingSource<F, V> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<Self>().hash(hasher);
    self.texel_to_reduce_unit.hash(hasher);
    self.reduce_unit_to_texel.hash(hasher);
  }
}

impl<V, F: ShaderTextureKind> FastDownSamplingIO<V> for CommonTextureFastDownSamplingSource<F, V> {
  fn root_size(&self) -> (u32, u32) {
    let input_size = self.target.desc.size;
    (input_size.width, input_size.height)
  }

  fn mip_level_count(&self) -> u32 {
    self.target.desc.mip_level_count
  }

  fn bind_first_stage_shader(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn FastDownSamplingIOFirstStageInvocation<V>> {
    Box::new(CommonTextureFastDownSamplingFirstStage {
      base_level: cx.bind_by(&self.base),
      levels: self.first_pass_writes.clone().map(|v| cx.bind_by(&v)),
      texel_to_reduce_unit: self.texel_to_reduce_unit,
      reduce_unit_to_texel: self.reduce_unit_to_texel,
    })
  }

  fn bind_first_stage_pass(&self, cx: &mut BindingBuilder) {
    for level in self.first_pass_writes.iter() {
      cx.bind(level);
    }
  }

  fn bind_second_stage_shader(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn FastDownSamplingIOSecondStageInvocation<V>> {
    Box::new(CommonTextureFastDownSamplingSecondStage {
      base_level: cx.bind_by(&self.second_pass_read),
      levels: self.second_pass_writes.clone().map(|v| cx.bind_by(&v)),
      texel_to_reduce_unit: self.texel_to_reduce_unit,
      reduce_unit_to_texel: self.reduce_unit_to_texel,
    })
  }

  fn bind_second_stage_pass(&self, cx: &mut BindingBuilder) {
    for level in self.second_pass_writes.iter() {
      cx.bind(level);
    }
  }
}

pub struct CommonTextureFastDownSamplingFirstStage<F: 'static, V: 'static> {
  base_level: BindingNode<ShaderStorageTextureR2D<F>>,
  levels: [BindingNode<ShaderStorageTextureW2D<F>>; 6],
  pub texel_to_reduce_unit:
    fn(BindingNode<ShaderStorageTextureR2D<F>>) -> Box<dyn SourceImageLoader<V>>,
  pub reduce_unit_to_texel:
    fn(BindingNode<ShaderStorageTextureW2D<F>>) -> Box<dyn SourceImageWriter<V>>,
}

impl<V, F> FastDownSamplingIOFirstStageInvocation<V>
  for CommonTextureFastDownSamplingFirstStage<F, V>
{
  fn get_root_loader_with_possible_write(&self) -> Box<dyn SourceImageLoader<V>> {
    (self.texel_to_reduce_unit)(self.base_level)
  }

  fn get_1_6_level_writer(&self, absolute_index: usize) -> Box<dyn SourceImageWriter<V>> {
    (self.reduce_unit_to_texel)(self.levels[absolute_index - 1])
  }
}

pub struct CommonTextureFastDownSamplingSecondStage<F: 'static, V: 'static> {
  base_level: BindingNode<ShaderStorageTextureR2D<F>>,
  levels: [BindingNode<ShaderStorageTextureW2D<F>>; 5],
  pub texel_to_reduce_unit:
    fn(BindingNode<ShaderStorageTextureR2D<F>>) -> Box<dyn SourceImageLoader<V>>,
  pub reduce_unit_to_texel:
    fn(BindingNode<ShaderStorageTextureW2D<F>>) -> Box<dyn SourceImageWriter<V>>,
}

impl<V, F> FastDownSamplingIOSecondStageInvocation<V>
  for CommonTextureFastDownSamplingSecondStage<F, V>
{
  fn get_level_6_loader(&self) -> Box<dyn SourceImageLoader<V>> {
    (self.texel_to_reduce_unit)(self.base_level)
  }

  fn get_7_12_level_writer(&self, absolute_index: usize) -> Box<dyn SourceImageWriter<V>> {
    (self.reduce_unit_to_texel)(self.levels[absolute_index - 1 - 6 - 1])
  }
}

pub trait SourceImageLoader<V: ShaderSizedValueNodeType> {
  fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<V>;

  fn down_sample_quad(&self, coord: Node<Vec2<u32>>, reducer: &dyn QuadReducer<V>) -> Node<V> {
    let loads = [vec2(0, 0), vec2(0, 1), vec2(1, 0), vec2(1, 1)].map(|offset| {
      // todo, boundary check?
      self.load_tex(coord + val(offset))
    });
    reducer.reduce(loads)
  }
}

pub trait SourceImageWriter<V: ShaderSizedValueNodeType> {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<V>);
}

impl<D, F> SourceImageLoader<ChannelOutputOf<F>> for BindingNode<ShaderTexture<D, F>>
where
  D: ShaderTextureDimension + SingleLayerTarget + DirectAccessTarget,
  F: ShaderTextureKind + SingleSampleTarget,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<ChannelOutputOf<F>> {
    self.load_texel(coord.into(), val(0))
  }
}

impl<A, D> SourceImageLoader<Vec4<f32>> for BindingNode<ShaderStorageTexture<A, D, f32>>
where
  D: ShaderTextureDimension + SingleLayerTarget + DirectAccessTarget,
  A: StorageTextureReadable,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<Vec4<f32>> {
    self.load_texel(coord.into())
  }
}

impl<A, D> SourceImageWriter<Vec4<f32>> for BindingNode<ShaderStorageTexture<A, D, f32>>
where
  D: ShaderTextureDimension + SingleLayerTarget + DirectAccessTarget,
  A: StorageTextureWriteable,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<Vec4<f32>>) {
    self.write_texel(coord.into(), value);
  }
}

pub struct FirstChannelLoader(pub BindingNode<ShaderStorageTextureR2D>);
impl SourceImageLoader<f32> for FirstChannelLoader {
  fn load_tex(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    self.0.load_texel(coord).x()
  }
}

pub struct SplatWriter(pub BindingNode<ShaderStorageTextureW2D>);

impl SourceImageWriter<f32> for SplatWriter {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<f32>) {
    self.0.write(coord, value.splat());
  }
}
