use crate::*;

pub type SceneTexture2D<S> = SceneTexture<<S as SceneContent>::Texture2D>;
pub type SceneTextureCube<S> = SceneTexture<<S as SceneContent>::TextureCube>;

pub struct SceneTexture<T> {
  pub content: Arc<RwLock<Identity<T>>>,
}

impl<T> SceneTexture<T> {
  pub fn new(source: T) -> Self {
    let content = Arc::new(RwLock::new(Identity::new(source)));
    Self { content }
  }

  pub fn mutate(&self, mutator: &dyn Fn(&mut T)) {
    let mut content = self.content.write().unwrap();

    mutator(&mut content);

    content.trigger_change()
  }
}

impl<T> Clone for SceneTexture<T> {
  fn clone(&self) -> Self {
    Self {
      content: self.content.clone(),
    }
  }
}
