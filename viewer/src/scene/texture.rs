use std::{cell::RefCell, rc::Rc};

use rendiation_texture::CubeTextureFace;
use rendiation_webgpu::*;

pub trait MaterialBindableResourceUpdate {
  type GPU;
  fn update<'a>(
    &self,
    gpu: &'a mut Option<Self::GPU>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> &'a mut Self::GPU;
}

impl MaterialBindableResourceUpdate for Box<dyn WebGPUTexture2dSource> {
  type GPU = WebGPUTexture2d;
  fn update<'a>(
    &self,
    gpu: &'a mut Option<Self::GPU>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> &'a mut Self::GPU {
    gpu.get_or_insert_with(|| {
      let source = self.as_ref();
      let desc = source.create_tex2d_desc(MipLevelCount::EmptyMipMap);

      WebGPUTexture2d::create(device, desc).upload_into(queue, source, 0)
    })
  }
}

impl MaterialBindableResourceUpdate for TextureCubeSource {
  type GPU = WebGPUTextureCube;
  fn update<'a>(
    &self,
    gpu: &'a mut Option<Self::GPU>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> &'a mut Self::GPU {
    gpu.get_or_insert_with(|| {
      let source = self.as_ref();
      let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);

      WebGPUTextureCube::create(device, desc)
        .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
        .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
        .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
        .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
        .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
        .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
    })
  }
}

pub type TextureCubeSource = [Box<dyn WebGPUTexture2dSource>; 6];

pub type SceneTexture2D = SceneTexture<Box<dyn WebGPUTexture2dSource>, WebGPUTexture2d>;
pub type SceneTextureCube = SceneTexture<TextureCubeSource, WebGPUTextureCube>;

pub struct SceneTexture<T, G> {
  pub content: Rc<RefCell<SceneTextureContent<T, G>>>,
}

impl<T, G> SceneTexture<T, G> {
  pub fn new(source: T) -> Self {
    let content = SceneTextureContent {
      source,
      gpu: None,
      on_changed: Vec::new(),
    };
    let content = Rc::new(RefCell::new(content));
    Self { content }
  }

  pub fn mutate(&self, mutator: &dyn Fn(&mut T)) {
    let mut content = self.content.borrow_mut();

    mutator(&mut content.source);

    content.gpu = None;

    let notifier_to_remove: Vec<_> = content
      .on_changed
      .iter()
      .enumerate()
      .filter_map(|(i, f)| (!f()).then(|| i))
      .collect();

    notifier_to_remove.iter().for_each(|&i| {
      content.on_changed.swap_remove(i)();
    });
  }
}

impl<T, G> Clone for SceneTexture<T, G> {
  fn clone(&self) -> Self {
    Self {
      content: self.content.clone(),
    }
  }
}

pub struct SceneTextureContent<T, G> {
  pub source: T,
  pub gpu: Option<G>,
  pub on_changed: Vec<Box<dyn Fn() -> bool>>,
}
