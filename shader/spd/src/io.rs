use crate::*;

pub trait SourceImageLoader<V: ShaderNodeType> {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<V>;
}

pub trait SourceImageWriter<V: ShaderNodeType> {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<V>);
}

impl<D, F> SourceImageLoader<ChannelOutputOf<F>> for BindingNode<ShaderTexture<D, F>>
where
  D: ShaderTextureDimension + SingleLayerTarget + DirectAccessTarget,
  F: ShaderTextureKind + SingleSampleTarget,
  Node<TextureSampleInputOf<D, u32>>: From<Node<Vec2<u32>>>,
{
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<ChannelOutputOf<F>> {
    self.load_texel(coord.into(), val(0))
  }
}

pub struct MSDepthLoader {
  pub mip_0: BindingNode<ShaderStorageTextureW2D>,
  pub ms_depth: BindingNode<ShaderMultiSampleDepthTexture2D>,
  pub scale: Node<Vec2<f32>>,
}

impl SourceImageLoader<f32> for MSDepthLoader {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    let depth_coord = coord.into_f32() * self.scale;
    let depth_coord = depth_coord.round().into_u32();

    let d1 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(0));
    let d2 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(1));
    let d3 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(2));
    let d4 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(3));

    let v = (d1 + d2 + d3 + d4) / val(4.); // todo fix me, this is wrong!
    self.mip_0.write_texel(coord, v.splat());
    v
  }
}

pub struct LoadFirstChannel {
  pub source: BindingNode<ShaderTexture2D>,
}
impl SourceImageLoader<f32> for LoadFirstChannel {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    self.source.load(coord).x()
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

pub struct SplatWriter(pub BindingNode<ShaderStorageTextureW2D>);

impl SourceImageWriter<f32> for SplatWriter {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<f32>) {
    self.0.write(coord, value.splat());
  }
}
