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
  pub second_pass_writes: [StorageTextureViewWriteonly2D<F>; 6],

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
          format: Some(target.desc.format.remove_srgb_suffix()),
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

    let second_pass_writes = std::array::from_fn(|index| {
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
      second_pass_writes,
      texel_to_reduce_unit,
      reduce_unit_to_texel,
    }
  }
}

impl<F: 'static, V: 'static> ShaderHashProvider for CommonTextureFastDownSamplingSource<F, V> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    hasher.hash_type::<Self>();
    hasher.hash(self.texel_to_reduce_unit);
    hasher.hash(self.reduce_unit_to_texel);
    hasher.hash(self.target.desc.format);
  }
}

impl<V, F> FastDownSamplingIO<V> for CommonTextureFastDownSamplingSource<F, V>
where
  F: ShaderTextureKind + StorageTextureChannelType,
{
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
    cx.bind(&self.base);
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
    cx.bind(&self.second_pass_read);
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
  levels: [BindingNode<ShaderStorageTextureW2D<F>>; 6],
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
    (self.reduce_unit_to_texel)(self.levels[absolute_index - 6 - 1])
  }
}

/// Clamp the out of bounds load coord to the edge in the shader code, instead of relying on the
/// out of bounds behavior of the shader runtime check.
///
/// Disabling this is not recommended, even if the wgpu shader runtime check is enabled. The WGSL
/// spec allows an out of bounds load to return any texel inside of the texture, or zero, and the
/// actual behavior differs across backends. With the runtime check enabled (as of wgpu 29), Metal
/// and Vulkan without robust image access clamp the coord to the edge, while Vulkan with robust
/// image access, DX12 and GL return zero, and GLES is not checked at all. In the browser it
/// depends on the implementation, Firefox follows the wgpu backend above, Chrome usually clamps.
///
/// The out of bounds lanes contribute to the valid output when one dimension has shrunk to 1
/// before the other (the tail levels of a non square pyramid), so without this check the result is
/// not consistent across backends. For the max reducer zero is harmless, for the min reducer
/// (reverse z depth) zero means the farthest depth, the result is still conservative but these
/// levels lose the culling ability, for the average reducer the result is wrong.
const ENABLE_LOAD_BOUND_CHECK: bool = true;

/// Skip the out of bounds write in the shader code, instead of relying on the out of bounds
/// behavior of the shader runtime check.
///
/// It is recommended to disable this when the wgpu shader runtime check is enabled (it is always
/// enabled in the browser). The WGSL spec guarantees that an out of bounds textureStore is not
/// executed, and naga documents that all the platforms it supports already discard out of bounds
/// image stores, so naga never injects any check for them. Disabling this saves a compare and a
/// branch for each write.
const ENABLE_WRITE_BOUND_CHECK: bool = true;

/// Clamp the load coord into the image if [ENABLE_LOAD_BOUND_CHECK], the size must be at least 1
/// in each dimension.
pub(crate) fn clamp_load_coord(coord: Node<Vec2<u32>>, size: Node<Vec2<u32>>) -> Node<Vec2<u32>> {
  if ENABLE_LOAD_BOUND_CHECK {
    coord.min(size - val(vec2(1, 1)))
  } else {
    coord
  }
}

/// The bound check is done by this crate itself, see `ENABLE_LOAD_BOUND_CHECK` and
/// `ENABLE_WRITE_BOUND_CHECK` for the reason and the switch.
pub trait SourceImageLoader<V: ShaderSizedValueNodeType> {
  /// The size of the image, it is at least 1 in each dimension.
  fn size(&self) -> Node<Vec2<u32>>;
  /// The coord is inside of the image if `ENABLE_LOAD_BOUND_CHECK`, otherwise it may be out of
  /// bounds and the shader runtime check takes over.
  fn load_in_bounds(&self, coord: Node<Vec2<u32>>) -> Node<V>;

  /// If `ENABLE_LOAD_BOUND_CHECK`, the coord outside of the image is clamped to the edge, so the
  /// edge texels are replicated.
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<V> {
    self.load_in_bounds(clamp_load_coord(coord, self.size()))
  }

  /// The input order of the reducer follows [QuadReducer].
  fn down_sample_quad(&self, coord: Node<Vec2<u32>>, reducer: &dyn QuadReducer<V>) -> Node<V> {
    let loads =
      [vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 1)].map(|offset| self.load(coord + val(offset)));
    reducer.reduce(loads)
  }
}

/// See [SourceImageLoader] for the bound check.
pub trait SourceImageWriter<V: ShaderSizedValueNodeType> {
  /// The size of the image, it is at least 1 in each dimension.
  fn size(&self) -> Node<Vec2<u32>>;
  /// The coord is inside of the image if `ENABLE_WRITE_BOUND_CHECK`, otherwise it may be out of
  /// bounds and the shader runtime check takes over.
  fn write_in_bounds(&self, coord: Node<Vec2<u32>>, value: Node<V>);

  /// The write outside of the image is skipped.
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<V>) {
    if ENABLE_WRITE_BOUND_CHECK {
      if_by(coord.less_than(self.size()).all(), || {
        self.write_in_bounds(coord, value);
      });
    } else {
      self.write_in_bounds(coord, value);
    }
  }
}

impl<D, F> SourceImageLoader<TexelOutputOf<F>> for BindingNode<ShaderTexture<D, F>>
where
  D: ShaderTextureDimension + SingleLayerTarget + D2LikeTextureType + DirectAccessTarget,
  F: ShaderTextureKind + SingleSampleTarget,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn size(&self) -> Node<Vec2<u32>> {
    self.texture_dimension_2d(None)
  }
  fn load_in_bounds(&self, coord: Node<Vec2<u32>>) -> Node<TexelOutputOf<F>> {
    self.load_texel(coord.into(), 0)
  }
}

impl<A, D> SourceImageLoader<Vec4<f32>> for BindingNode<ShaderStorageTexture<A, D, f32>>
where
  D: ShaderTextureDimension + SingleLayerTarget + D2LikeTextureType + DirectAccessTarget,
  A: StorageTextureReadable,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn size(&self) -> Node<Vec2<u32>> {
    self.texture_dimension_2d()
  }
  fn load_in_bounds(&self, coord: Node<Vec2<u32>>) -> Node<Vec4<f32>> {
    self.load_texel(coord.into())
  }
}

impl<A, D> SourceImageWriter<Vec4<f32>> for BindingNode<ShaderStorageTexture<A, D, f32>>
where
  D: ShaderTextureDimension + SingleLayerTarget + D2LikeTextureType + DirectAccessTarget,
  A: StorageTextureWriteable,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn size(&self) -> Node<Vec2<u32>> {
    self.texture_dimension_2d()
  }
  fn write_in_bounds(&self, coord: Node<Vec2<u32>>, value: Node<Vec4<f32>>) {
    self.write_texel(coord.into(), value);
  }
}

pub struct FirstChannelLoader(pub BindingNode<ShaderStorageTextureR2D>);
impl SourceImageLoader<f32> for FirstChannelLoader {
  fn size(&self) -> Node<Vec2<u32>> {
    self.0.texture_dimension_2d()
  }
  fn load_in_bounds(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    self.0.load_texel(coord).x()
  }
}

pub struct SplatWriter(pub BindingNode<ShaderStorageTextureW2D>);

impl SourceImageWriter<f32> for SplatWriter {
  fn size(&self) -> Node<Vec2<u32>> {
    self.0.texture_dimension_2d()
  }
  fn write_in_bounds(&self, coord: Node<Vec2<u32>>, value: Node<f32>) {
    self.0.write_texel(coord, value.splat());
  }
}
