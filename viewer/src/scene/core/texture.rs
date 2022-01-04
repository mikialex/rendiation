use std::{cell::RefCell, rc::Rc};

use rendiation_webgpu::{WebGPUTexture2dSource, WebGPUTextureCube, WebGPUTexture2d};

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