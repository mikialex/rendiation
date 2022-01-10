use std::{cell::RefCell, rc::Rc};

use rendiation_webgpu::WebGPUTexture2dSource;

use crate::ResourceWrapped;

pub type TextureCubeSource = [Box<dyn WebGPUTexture2dSource>; 6];

pub type SceneTexture2D = SceneTexture<Box<dyn WebGPUTexture2dSource>>;
pub type SceneTextureCube = SceneTexture<TextureCubeSource>;

pub struct SceneTexture<T> {
  pub content: Rc<RefCell<ResourceWrapped<T>>>,
}

impl<T> SceneTexture<T> {
  pub fn new(source: T) -> Self {
    let content = Rc::new(RefCell::new(ResourceWrapped::new(source)));
    Self { content }
  }

  pub fn mutate(&self, mutator: &dyn Fn(&mut T)) {
    // let mut content = self.content.borrow_mut();

    // mutator(&mut content.source);

    // content.gpu = None;

    // let notifier_to_remove: Vec<_> = content
    //   .on_changed
    //   .iter()
    //   .enumerate()
    //   .filter_map(|(i, f)| (!f()).then(|| i))
    //   .collect();

    // notifier_to_remove.iter().for_each(|&i| {
    //   content.on_changed.swap_remove(i)();
    // });
  }
}

impl<T> Clone for SceneTexture<T> {
  fn clone(&self) -> Self {
    Self {
      content: self.content.clone(),
    }
  }
}
