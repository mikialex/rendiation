use rendiation_texture::CubeTextureFace;

use crate::*;

pub enum TextureGPUChange {
  Reference2D(GPU2DTextureView),
  ReferenceCube(GPUCubeTextureView),
  Content,
}

impl TextureGPUChange {
  fn to_render_component_delta(&self) -> RenderComponentDelta {
    match self {
      TextureGPUChange::Reference2D(_) => RenderComponentDelta::ContentRef,
      TextureGPUChange::ReferenceCube(_) => RenderComponentDelta::ContentRef,
      TextureGPUChange::Content => RenderComponentDelta::ContentRef,
    }
  }
}

pub struct ReactiveGPU2DTextureSignal {
  inner: EventSource<TextureGPUChange>,
  gpu: GPU2DTextureView,
}

pub type Texture2dRenderComponentDeltaStream = impl Stream<Item = RenderComponentDelta>;

pub type ReactiveGPU2DTextureView =
  impl AsRef<ReactiveGPU2DTextureSignal> + Stream<Item = TextureGPUChange>;

impl ReactiveGPU2DTextureSignal {
  // todo , fix send sync in webgpu resource first
  // pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = TextureGPUChange> {
  //   // create channel here, and send the init value
  //   let s = self
  //     .inner
  //     .listen_by(TextureGPUChange::to_render_component_delta);

  //   s
  // }
  pub fn create_gpu_texture_com_delta_stream(&self) -> Texture2dRenderComponentDeltaStream {
    self.inner.listen_by(
      TextureGPUChange::to_render_component_delta,
      RenderComponentDelta::ContentRef,
    )
  }
}

pub struct ReactiveGPUCubeTextureSignal {
  inner: EventSource<TextureGPUChange>,
  gpu: GPUCubeTextureView,
}

pub type TextureCubeRenderComponentDeltaStream = impl Stream<Item = RenderComponentDelta>;

pub type ReactiveGPUCubeTextureView =
  impl AsRef<ReactiveGPUCubeTextureSignal> + Stream<Item = TextureGPUChange>;

impl ReactiveGPUCubeTextureSignal {
  // todo , fix send sync in webgpu resource first
  // pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = TextureGPUChange> {
  //   // create channel here, and send the init value
  //   let s = self
  //     .inner
  //     .listen_by(TextureGPUChange::to_render_component_delta);

  //   s
  // }
  pub fn create_gpu_texture_com_delta_stream(&self) -> TextureCubeRenderComponentDeltaStream {
    self.inner.listen_by(
      TextureGPUChange::to_render_component_delta,
      RenderComponentDelta::ContentRef,
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_texture2d(
    &self,
    tex: &SceneTexture2D,
  ) -> (GPU2DTextureView, Texture2dRenderComponentDeltaStream) {
    let mut texture_2d = self.texture_2d.write().unwrap();
    let cache = texture_2d.get_or_insert_with(tex.id(), || {
      let gpu_tex = self.gpu.create_gpu_texture2d(tex);

      let gpu_tex = ReactiveGPU2DTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let weak_tex = tex.downgrade();

      let updater = move |_delta, gpu_tex: &mut ReactiveGPU2DTextureSignal| {
        if let Some(tex) = weak_tex.upgrade() {
          let recreated = gpu_clone.create_gpu_texture2d(&tex);
          gpu_tex.gpu = recreated.clone();
          gpu_tex
            .inner
            .emit(&TextureGPUChange::Reference2D(gpu_tex.gpu.clone()));
          TextureGPUChange::Reference2D(recreated).into()
        } else {
          None
        }
      };

      tex.listen_by(any_change).fold_signal(gpu_tex, updater)
    });

    let tex = cache.as_ref().gpu.clone();
    let tex_s = cache.as_ref().create_gpu_texture_com_delta_stream();
    (tex, tex_s)
  }

  pub fn build_texture_sampler_pair(&self, t: &Texture2DWithSamplingData) -> GPUTextureSamplerPair {
    let sampler = GPUSampler::create(t.sampler.into(), &self.gpu.device);
    let sampler = sampler.create_default_view();

    let (texture, _) = self.get_or_create_reactive_gpu_texture2d(&t.texture);

    GPUTextureSamplerPair { texture, sampler }
  }

  pub fn get_or_create_reactive_gpu_texture_cube(
    &self,
    tex: &SceneTextureCube,
  ) -> (GPUCubeTextureView, TextureCubeRenderComponentDeltaStream) {
    let mut texture_cube = self.texture_cube.write().unwrap();
    let cache = texture_cube.get_or_insert_with(tex.id(), || {
      let gpu_tex = self.gpu.create_gpu_texture_cube(tex);

      let gpu_tex = ReactiveGPUCubeTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let weak_tex = tex.downgrade();

      let updater = move |_delta, gpu_tex: &mut ReactiveGPUCubeTextureSignal| {
        if let Some(tex) = weak_tex.upgrade() {
          let recreated = gpu_clone.create_gpu_texture_cube(&tex);
          gpu_tex.gpu = recreated.clone();
          gpu_tex
            .inner
            .emit(&TextureGPUChange::ReferenceCube(gpu_tex.gpu.clone()));
          TextureGPUChange::ReferenceCube(recreated).into()
        } else {
          None
        }
      };

      tex.listen_by(any_change).fold_signal(gpu_tex, updater)
    });

    let tex = cache.as_ref().gpu.clone();
    let tex_s = cache.as_ref().create_gpu_texture_com_delta_stream();
    (tex, tex_s)
  }
}

// resource creation ==>

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

impl ResourceGPUCtx {
  fn create_gpu_texture2d(&self, tex: &SceneTexture2D) -> GPU2DTextureView {
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

  fn create_gpu_texture_cube(&self, tex: &SceneTextureCube) -> GPUCubeTextureView {
    let texture = &tex.read();
    if let Some(t) = as_2d_source(&texture.faces[0]) {
      let source = &texture.faces;
      let desc = t.create_cube_desc(MipLevelCount::EmptyMipMap);
      let queue = &self.queue;

      let gpu_texture = GPUTexture::create(desc, &self.device);
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
      let tex: GPUCubeTexture = create_fallback_empty_cube_texture(&self.device);
      tex.create_default_view().try_into().unwrap()
    }
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
    device,
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
    device,
  )
  .try_into()
  .unwrap()
}

// texture sampler pair utils ==>

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
