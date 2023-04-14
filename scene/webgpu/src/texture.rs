use rendiation_texture::CubeTextureFace;

use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: GPU2DTextureView,
  pub sampler: GPUSamplerView,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx, group: impl Into<usize> + Copy) {
    ctx.binding.bind(&self.texture, group);
    ctx.binding.bind(&self.sampler, group);
  }

  pub fn uniform_and_sample(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    group: impl Into<usize> + Copy,
    position: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = binding.uniform_by(&self.texture, group);
    let sampler = binding.uniform_by(&self.sampler, group);
    texture.sample(sampler, position)
  }
}

pub fn build_texture_sampler_pair(
  t: &Texture2DWithSamplingData,
  gpu: &GPU,
  res: &mut GPUResourceSubCache,
) -> GPUTextureSamplerPair {
  let sampler = GPUSampler::create(t.sampler.into(), &gpu.device);
  let sampler = sampler.create_default_view();

  let ctx = TextureBuildCtx {
    gpu,
    mipmap_gen: &res.mipmap_gen,
  };

  let texture = res.texture_2ds.get_with_update(&t.texture, &ctx).0.clone();
  GPUTextureSamplerPair { texture, sampler }
}

pub struct TextureBuildCtx<'a> {
  gpu: &'a GPU,
  mipmap_gen: &'a Rc<RefCell<MipMapTaskManager>>,
}

pub struct Wrapped<T>(pub T);

impl SceneItemReactiveSimpleMapping<Wrapped<GPU2DTextureView>> for SceneTexture2D {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx<'a> = TextureBuildCtx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (Wrapped<GPU2DTextureView>, Self::ChangeStream) {
    let source = self.read();
    let texture = create_texture2d(self, ctx);

    let change = source.listen_by(any_change);
    (Wrapped(texture), change)
  }
}

fn as_2d_source(tex: &SceneTexture2DType) -> Option<&dyn WebGPU2DTextureSource> {
  match tex {
    SceneTexture2DType::RGBAu8(tex) => Some(tex),
    SceneTexture2DType::RGBu8(tex) => Some(tex),
    SceneTexture2DType::RGBAf32(tex) => Some(tex),
    SceneTexture2DType::Foreign(tex) => tex
      .downcast_ref::<Box<dyn WebGPU2DTextureSource>>()
      .map(|t| t.as_ref()),
    _ => None,
  }
}

fn create_texture2d(tex: &SceneTexture2D, ctx: &TextureBuildCtx) -> GPU2DTextureView {
  let texture = &tex.read();
  let texture = as_2d_source(texture);
  let gpu = ctx.gpu;

  let gpu_texture = if let Some(texture) = texture {
    let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
    let gpu_texture = GPUTexture::create(desc, &gpu.device);
    let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
    gpu_texture.upload_into(&gpu.queue, texture, 0)
  } else {
    create_fallback_empty_texture(&gpu.device)
  };

  ctx.mipmap_gen.borrow_mut().request_mipmap_gen(&gpu_texture);
  gpu_texture.create_default_view().try_into().unwrap()
}

#[derive(Clone)]
struct GPU2DTextureViewContent(GPU2DTextureView);

clone_self_incremental!(GPU2DTextureViewContent);

// todo
unsafe impl Send for GPU2DTextureViewContent {}
unsafe impl Sync for GPU2DTextureViewContent {}

pub struct ReactiveGPU2DTextureSignal {
  inner: Identity<GPU2DTextureViewContent>,
}

// pub type ReactiveGPU2DTextureViewBuilder =
//   impl StreamForkable<Item = GPUResourceChange<GPU2DTextureView>>;
// pub type ReactiveGPU2DTextureView = <ReactiveGPU2DTextureViewBuilder as StreamForkable>::Stream;
pub type ReactiveGPU2DTextureView = impl AsRef<ReactiveGPU2DTextureSignal>;

impl ShareBindableResource {
  pub fn get_or_create_reactive_gpu_texture2d(
    &mut self,
    tex: &SceneTexture2D,
  ) -> &mut ReactiveGPU2DTextureSignal {
    self.texture_2d.get_or_insert_with(tex.id(), || {
      let gpu_tex = self.gpu.create_gpu_texture2d(tex);

      let gpu_tex = GPU2DTextureViewContent(gpu_tex);
      let gpu_tex = Identity::new(gpu_tex);

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let tex_clone = tex.clone();

      let updater = move |delta, gpu_tex: &mut ReactiveGPU2DTextureSignal| {
        let recreated = gpu_clone.create_gpu_texture2d(&tex_clone);
        *gpu_tex = recreated.clone();
        // GPUResourceChange::Reference(recreated)
      };

      tex.listen_by(any_change).fold_signal(gpu_tex, updater)
    })
  }
}

impl ResourceGPUCtx {
  pub fn create_gpu_texture2d(&self, tex: &SceneTexture2D) -> GPU2DTextureView {
    let texture = &tex.read();
    let texture = as_2d_source(texture);

    let gpu_texture = if let Some(texture) = texture {
      let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
      let gpu_texture = GPUTexture::create(desc, &self.device);
      let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
      gpu_texture.upload_into(&self.queue, texture, 0)
    } else {
      create_fallback_empty_texture(&self.device)
    };

    self
      .mipmap_gen
      .borrow_mut()
      .request_mipmap_gen(&gpu_texture);
    gpu_texture.create_default_view().try_into().unwrap()
  }
}

impl SceneItemReactiveSimpleMapping<Wrapped<GPUCubeTextureView>> for SceneTextureCube {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx<'a> = TextureBuildCtx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (Wrapped<GPUCubeTextureView>, Self::ChangeStream) {
    let source = self.read();
    let texture = create_texture_cube(self, ctx);

    let change = source.listen_by(any_change);
    (Wrapped(texture), change)
  }
}

fn create_texture_cube(tex: &SceneTextureCube, ctx: &TextureBuildCtx) -> GPUCubeTextureView {
  let gpu = ctx.gpu;
  let texture = &tex.read();
  if let Some(t) = as_2d_source(&texture.faces[0]) {
    let source = &texture.faces;
    let desc = t.create_cube_desc(MipLevelCount::EmptyMipMap);
    let queue = &gpu.queue;

    let gpu_texture = GPUTexture::create(desc, &gpu.device);
    let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();

    #[rustfmt::skip]
    gpu_texture
      .upload(queue, as_2d_source(&source[0]).unwrap(), CubeTextureFace::PositiveX, 0)
      .upload(queue, as_2d_source(&source[1]).unwrap(), CubeTextureFace::NegativeX, 0)
      .upload(queue, as_2d_source(&source[2]).unwrap(), CubeTextureFace::PositiveY, 0)
      .upload(queue, as_2d_source(&source[3]).unwrap(), CubeTextureFace::NegativeY, 0)
      .upload(queue, as_2d_source(&source[4]).unwrap(), CubeTextureFace::PositiveZ, 0)
      .upload(queue, as_2d_source(&source[5]).unwrap(), CubeTextureFace::NegativeZ, 0)
      .create_default_view()
      .try_into()
      .unwrap()
  } else {
    let tex: GPUCubeTexture = create_fallback_empty_cube_texture(&gpu.device);
    tex.create_default_view().try_into().unwrap()
  }
}

fn create_fallback_empty_texture(device: &GPUDevice) -> GPU2DTexture {
  GPUTexture::create(
    webgpu::TextureDescriptor {
      label: "unimplemented default texture".into(),
      size: webgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: webgpu::TextureDimension::D2,
      format: webgpu::TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: webgpu::TextureUsages::all(),
    },
    &device,
  )
  .try_into()
  .unwrap()
}

fn create_fallback_empty_cube_texture(device: &GPUDevice) -> GPUCubeTexture {
  GPUTexture::create(
    webgpu::TextureDescriptor {
      label: "unimplemented default texture".into(),
      size: webgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: webgpu::TextureDimension::D2,
      format: webgpu::TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: webgpu::TextureUsages::all(),
    },
    &device,
  )
  .try_into()
  .unwrap()
}
