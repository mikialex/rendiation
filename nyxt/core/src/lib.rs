use std::{cell::RefCell, rc::Rc, rc::Weak};

use rendiation_ral::*;
use rendiation_scenegraph::{
  default_impl::SceneNodeData, DrawcallHandle, HtmlCanvasElement, Scene, SceneNodeHandle,
};
use rendiation_webgl::{WebGL, WebGLRenderer};
use wasm_bindgen::prelude::*;

pub mod geometry;
pub mod scene;

pub use geometry::*;
pub use scene::*;

pub type GFX = WebGL;

#[wasm_bindgen]
pub struct NyxtViewer {
  inner: Rc<RefCell<NyxtViewerInner>>,
}

pub struct NyxtViewerInner {
  pub renderer: WebGLRenderer,
  pub resource: ResourceManager<GFX>,
  pub scene: Scene<GFX>,
}

pub trait NyxtViewerHandle: Copy {
  type Item;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item;
  fn free(self, inner: &mut NyxtViewerInner);
}

pub trait NyxtViewerMutableHandle: NyxtViewerHandle {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item;
}

pub struct NyxtViewerHandledObject<Handle: NyxtViewerHandle> {
  pub handle: Handle,
  pub inner: Weak<RefCell<NyxtViewerInner>>,
}

impl<Handle: NyxtViewerHandle> NyxtViewerHandledObject<Handle> {
  pub fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut NyxtViewerInner) -> T) -> T {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    let mut inner = inner.borrow_mut();
    mutator(&mut inner)
  }
}

#[wasm_bindgen]
impl NyxtViewer {
  #[wasm_bindgen(constructor)]
  pub fn new(canvas: HtmlCanvasElement) -> Self {
    console_error_panic_hook::set_once();
    Self {
      inner: Rc::new(RefCell::new(NyxtViewerInner {
        renderer: WebGLRenderer::new(canvas),
        resource: ResourceManager::new(),
        scene: Scene::new(),
      })),
    }
  }
}

impl NyxtViewer {
  pub fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut NyxtViewerInner) -> T) -> T {
    let mut inner = self.inner.borrow_mut();
    mutator(&mut inner)
  }
}

impl<Handle: NyxtViewerMutableHandle> NyxtViewerHandledObject<Handle> {
  pub fn mutate_item<T>(&self, mutator: impl FnOnce(&mut Handle::Item) -> T) -> T {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    let mut inner = inner.borrow_mut();
    let item = self.handle.get_mut(&mut inner);
    mutator(item)
  }
}

impl NyxtViewer {
  pub fn make_handle_object<T: NyxtViewerHandle>(&self, handle: T) -> NyxtViewerHandledObject<T> {
    let inner = Rc::downgrade(&self.inner);
    NyxtViewerHandledObject { handle, inner }
  }
}

#[derive(Copy, Clone)]
pub struct UniformHandleWrap<T>(pub UniformHandle<GFX, T>);

impl<T: Copy + 'static> NyxtViewerHandle for UniformHandleWrap<T> {
  type Item = T;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.resource.bindable.uniform_buffers.get_data(self.0)
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.bindable.uniform_buffers.delete(self.0)
  }
}
impl<T: Copy + 'static> NyxtViewerMutableHandle for UniformHandleWrap<T> {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.resource.bindable.uniform_buffers.mutate(self.0)
  }
}

impl NyxtViewerHandle for DrawcallHandle<GFX> {
  type Item = Drawcall<GFX>;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.scene.drawcalls.get(self).unwrap()
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.scene.drawcalls.remove(self);
  }
}

impl NyxtViewerHandle for SceneNodeHandle<GFX> {
  type Item = SceneNodeData<GFX>;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.scene.get_node(self).data()
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.scene.free_node(self)
  }
}
impl NyxtViewerMutableHandle for SceneNodeHandle<GFX> {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.scene.get_node_mut(self).data_mut()
  }
}
