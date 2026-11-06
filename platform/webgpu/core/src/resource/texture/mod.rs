mod d2;
pub use check::*;
pub use d2::*;
mod cube;
mod storage;
pub use storage::*;
mod check;

use crate::*;

pub type GPUTexture = ResourceRc<GPURawTexture>;
pub type GPUTextureView = ResourceViewRc<GPURawTexture>;

/// The wgpu texture is wrapped to make sure the view creation is counted, see
/// [GPUCallCountReport].
#[derive(Clone)]
pub struct GPURawTexture {
  gpu: gpu::Texture,
  counters: Arc<GPUCallCounters>,
}

impl GPURawTexture {
  /// wrap a texture created by the device or outside, for example the texture created by
  /// [GPUDevice::create_texture]. The creation of the outside texture is not counted.
  pub fn new(texture: gpu::Texture, device: &GPUDevice) -> Self {
    Self {
      gpu: texture,
      counters: device.inner.counters.clone(),
    }
  }

  /// Access the underlying wgpu texture.
  ///
  /// The calls made through the raw texture(create_view and destroy) are not counted in
  /// [GPUCallCountReport]. It's mainly used to build the copy parameters.
  pub fn gpu(&self) -> &gpu::Texture {
    &self.gpu
  }

  pub fn as_image_copy(&self) -> gpu::TexelCopyTextureInfo<'_> {
    self.gpu.as_image_copy()
  }

  pub fn size(&self) -> gpu::Extent3d {
    self.gpu.size()
  }

  pub fn width(&self) -> u32 {
    self.gpu.width()
  }

  pub fn height(&self) -> u32 {
    self.gpu.height()
  }

  pub fn depth_or_array_layers(&self) -> u32 {
    self.gpu.depth_or_array_layers()
  }

  pub fn mip_level_count(&self) -> u32 {
    self.gpu.mip_level_count()
  }

  pub fn sample_count(&self) -> u32 {
    self.gpu.sample_count()
  }

  pub fn dimension(&self) -> gpu::TextureDimension {
    self.gpu.dimension()
  }

  pub fn format(&self) -> gpu::TextureFormat {
    self.gpu.format()
  }

  pub fn usage(&self) -> gpu::TextureUsages {
    self.gpu.usage()
  }
}

impl Resource for GPURawTexture {
  type Descriptor = gpu::TextureDescriptor<'static>;

  type View = gpu::TextureView;

  type ViewDescriptor = gpu::TextureViewDescriptor<'static>;

  fn create_view(&self, desc: &Self::ViewDescriptor) -> Self::View {
    self.counters.device.create_texture_view.inc();
    self.gpu.create_view(desc)
  }
}

impl InitResourceByAllocation for GPURawTexture {
  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    Self::new(device.create_texture(desc), device)
  }
}

impl BindableResourceProvider for GPUTextureView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::TextureView(self.clone())
  }
}
impl BindableResourceView for gpu::TextureView {
  fn as_bindable(&self) -> gpu::BindingResource<'_> {
    gpu::BindingResource::TextureView(self)
  }
}

#[derive(Debug)]
pub struct GPUTypedTexture<D, F> {
  pub typed_desc: PhantomData<(D, F)>,
  pub texture: GPUTexture,
}

impl<D, F> TryFrom<GPUTexture> for GPUTypedTexture<D, F> {
  type Error = &'static str;

  fn try_from(texture: GPUTexture) -> Result<Self, Self::Error> {
    // if $check {
    //   Ok(Self($var))
    // } else {
    //   Err("raw texture not a 1d")
    // }
    Ok(Self {
      typed_desc: PhantomData,
      texture,
    })
  }
}

impl<D, F> Clone for GPUTypedTexture<D, F> {
  fn clone(&self) -> Self {
    Self {
      typed_desc: self.typed_desc,
      texture: self.texture.clone(),
    }
  }
}

impl<D, F> PartialEq for GPUTypedTexture<D, F> {
  fn eq(&self, other: &Self) -> bool {
    self.texture == other.texture
  }
}

impl<D, F> Deref for GPUTypedTexture<D, F> {
  type Target = GPUTexture;

  fn deref(&self) -> &Self::Target {
    &self.texture
  }
}

pub type GPU1DTexture = GPUTypedTexture<TextureDimension1, f32>;
pub type GPU2DTexture = GPUTypedTexture<TextureDimension2, f32>;
pub type GPU3DTexture = GPUTypedTexture<TextureDimension3, f32>;

pub type GPUCubeTexture = GPUTypedTexture<TextureDimensionCube, f32>;

impl GPU2DTexture {
  pub fn size_2d(&self) -> Size {
    let size = self.size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

#[derive(Debug)]
pub struct GPUTypedTextureView<D, F> {
  pub typed_desc: PhantomData<(D, F)>,
  pub texture: GPUTextureView,
}

impl<D, F> Clone for GPUTypedTextureView<D, F> {
  fn clone(&self) -> Self {
    Self {
      typed_desc: self.typed_desc,
      texture: self.texture.clone(),
    }
  }
}

impl<D, F> PartialEq for GPUTypedTextureView<D, F> {
  fn eq(&self, other: &Self) -> bool {
    self.texture == other.texture
  }
}

impl<D, F> TryFrom<GPUTextureView> for GPUTypedTextureView<D, F>
where
  D: DimensionDynamicViewCheck,
  F: TextureFormatDynamicCheck,
{
  type Error = &'static str; // todo, improve error report

  fn try_from(texture: GPUTextureView) -> Result<Self, Self::Error> {
    if !D::check(&texture.desc, &texture.resource.desc) {
      return Err("texture dimension mismatch");
    }

    if !F::check(
      &texture.desc.format.unwrap_or(texture.resource.desc.format),
      texture.desc.aspect,
      texture.resource.desc.sample_count,
    ) {
      return Err("texture format mismatch");
    }

    Ok(Self {
      typed_desc: PhantomData,
      texture,
    })
  }
}

impl<D, F> Deref for GPUTypedTextureView<D, F> {
  type Target = GPUTextureView;

  fn deref(&self) -> &Self::Target {
    &self.texture
  }
}

impl<D, F> CacheAbleBindingSource for GPUTypedTextureView<D, F> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

pub type GPU1DTextureView = GPUTypedTextureView<TextureDimension1, f32>;
pub type GPU2DTextureView = GPUTypedTextureView<TextureDimension2, f32>;
pub type GPU2DArrayTextureView = GPUTypedTextureView<TextureDimension2Array, f32>;
pub type GPUCubeTextureView = GPUTypedTextureView<TextureDimensionCube, f32>;
pub type GPUCubeArrayTextureView = GPUTypedTextureView<TextureDimensionCubeArray, f32>;
pub type GPU3DTextureView = GPUTypedTextureView<TextureDimension3, f32>;

pub type GPU2DDepthTextureView = GPUTypedTextureView<TextureDimension2, TextureSampleDepth>;
pub type GPU2DArrayDepthTextureView =
  GPUTypedTextureView<TextureDimension2Array, TextureSampleDepth>;
pub type GPUCubeDepthTextureView = GPUTypedTextureView<TextureDimensionCube, TextureSampleDepth>;
pub type GPUCubeArrayDepthTextureView =
  GPUTypedTextureView<TextureDimensionCubeArray, TextureSampleDepth>;

pub type GPU2DMultiSampleTextureView = GPUTypedTextureView<TextureDimension2, MultiSampleOf<f32>>;
pub type GPU2DMultiSampleDepthTextureView =
  GPUTypedTextureView<TextureDimension2, MultiSampleOf<TextureSampleDepth>>;

impl<F> GPUTypedTextureView<TextureDimension2, F> {
  pub fn size(&self) -> Size {
    self.texture.size_assume_2d()
  }
}

impl GPUTextureView {
  pub fn size_assume_2d(&self) -> Size {
    let size = self
      .resource
      .desc
      .size
      .mip_level_size(self.desc.base_mip_level, gpu::TextureDimension::D2);
    GPUTextureSize::from_gpu_size(size)
  }
}
