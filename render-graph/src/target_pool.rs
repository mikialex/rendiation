use crate::{ContentProvider, RenderGraphBackend, RenderGraphNodeHandle, TargetNodeData};
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderTargetSize(pub NonZeroUsize, pub NonZeroUsize);

impl Default for RenderTargetSize {
  fn default() -> Self {
    Self::new(5, 5)
  }
}

impl RenderTargetSize {
  pub fn new(width: usize, height: usize) -> Self {
    Self(
      NonZeroUsize::new(width).unwrap(),
      NonZeroUsize::new(height).unwrap(),
    )
  }
  pub fn to_tuple(&self) -> (usize, usize) {
    (self.0.get(), self.1.get())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderTargetFormatKey<T> {
  pub size: RenderTargetSize,
  pub format: T,
}

impl<T> RenderTargetFormatKey<T> {
  pub fn default_with_format(format: T) -> Self {
    Self {
      size: RenderTargetSize::default(),
      format,
    }
  }
}

pub struct RenderTargetTypePooling<T: RenderGraphBackend> {
  key: RenderTargetFormatKey<T::RenderTargetFormatKey>,
  available: Vec<T::RenderTarget>,
}

impl<T: RenderGraphBackend> RenderTargetTypePooling<T> {
  pub fn request(&mut self, renderer: &T::Renderer) -> T::RenderTarget {
    if self.available.len() == 0 {
      self
        .available
        .push(T::create_render_target(renderer, &self.key))
    }
    self.available.pop().unwrap()
  }

  pub fn return_back(&mut self, target: T::RenderTarget) {
    self.available.push(target)
  }
}

pub struct RenderTargetPool<T: RenderGraphBackend, U: ContentProvider<T>> {
  cached: HashMap<RenderTargetFormatKey<T::RenderTargetFormatKey>, RenderTargetTypePooling<T>>,
  active_targets: HashMap<RenderGraphNodeHandle<T, U>, T::RenderTarget>,
}

impl<T: RenderGraphBackend, U: ContentProvider<T>> RenderTargetPool<T, U> {
  pub fn new() -> Self {
    Self {
      cached: HashMap::new(),
      active_targets: HashMap::new(),
    }
  }

  pub fn clear_all(&mut self, renderer: &T::Renderer) {
    if self.active_targets.len() > 0 {
      panic!("some target still in use")
    }
    self.cached.drain().for_each(|(_, p)| {
      p.available
        .into_iter()
        .for_each(|t| T::dispose_render_target(renderer, t))
    })
  }

  fn get_pool(
    &mut self,
    key: &RenderTargetFormatKey<T::RenderTargetFormatKey>,
  ) -> &mut RenderTargetTypePooling<T> {
    if !self.cached.contains_key(&key) {
      self.cached.insert(
        key.clone(),
        RenderTargetTypePooling {
          key: key.clone(),
          available: Vec::new(),
        },
      );
    }

    self.cached.get_mut(&key).unwrap()

    // is clone expensive?, need profile?
    // https://stackoverflow.com/questions/51542024/how-do-i-use-the-entry-api-with-an-expensive-key-that-is-only-constructed-if-the
    // wtf why this is not stable??

    // self
    //   .cached
    //   .entry(key.clone())
    //   .or_insert_with(|| RenderTargetTypePooling {
    //     key: key.clone(),
    //     available: Vec::new(),
    //   })
  }

  /// get a RenderTarget from pool,if there is no fbo meet the config, create a new one, and pool it
  pub fn request_render_target(
    &mut self,
    node_handle: RenderGraphNodeHandle<T, U>,
    data: &TargetNodeData<T>,
    renderer: &T::Renderer,
  ) -> &T::RenderTarget {
    let target = self.get_pool(&data.format).request(renderer);
    self.active_targets.entry(node_handle).or_insert(target)
  }

  /// return a framebuffer that maybe request before, which will be pooling and reused
  pub fn return_render_target(
    &mut self,
    node_handle: RenderGraphNodeHandle<T, U>,
    data: &TargetNodeData<T>,
  ) {
    let target = self.active_targets.remove(&node_handle).unwrap();
    self.get_pool(&data.format).return_back(target);
  }
}
