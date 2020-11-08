use std::{cell::RefCell, rc::Rc, rc::Weak};

use rendiation_ral::*;
use rendiation_render_entity::Camera;
use rendiation_scenegraph::{
  default_impl::SceneNodeData, DrawcallHandle, Scene, SceneDrawcallList, SceneNodeHandle,
};
use rendiation_webgl::{HtmlCanvasElement, WebGL, WebGLRenderer};
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
  cached_drawcall_list: SceneDrawcallList<GFX>,
  pub camera: Camera,
}

pub trait NyxtViewerHandle: Copy {
  type Item;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item;
  fn free(self, inner: &mut NyxtViewerInner);
}

pub trait NyxtViewerMutableHandle: NyxtViewerHandle {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item;
}

#[derive(Clone)]
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

  pub fn clone_viewer(&self) -> NyxtViewer {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    NyxtViewer { inner }
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

#[wasm_bindgen]
impl NyxtViewer {
  #[wasm_bindgen(constructor)]
  pub fn new(canvas: HtmlCanvasElement) -> Self {
    console_error_panic_hook::set_once();
    let mut resource = ResourceManager::new();
    let scene = Scene::new(&mut resource);
    Self {
      inner: Rc::new(RefCell::new(NyxtViewerInner {
        renderer: WebGLRenderer::new(canvas),
        resource,
        scene,
        cached_drawcall_list: SceneDrawcallList::new(),
        camera: Camera::new(),
      })),
    }
  }

  #[wasm_bindgen]
  pub fn get_root(&self) -> SceneNodeWASM {
    SceneNodeWASM {
      inner: self.make_handle_object(self.mutate_inner(|inner| inner.scene.get_root().handle())),
    }
  }

  #[wasm_bindgen]
  pub fn render(&self) {
    self.mutate_inner(|viewer| {
      let resource = &mut viewer.resource;
      let scene = &mut viewer.scene;
      let renderer = &mut viewer.renderer;
      let camera = &mut viewer.camera;

      let list = scene.update(resource, camera, &mut viewer.cached_drawcall_list);
      resource.maintain_gpu(renderer);

      list.render(renderer, scene, resource);
    });
  }
}

impl NyxtViewer {
  pub fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut NyxtViewerInner) -> T) -> T {
    let mut inner = self.inner.borrow_mut();
    mutator(&mut inner)
  }

  pub fn make_handle_object<T: NyxtViewerHandle>(&self, handle: T) -> NyxtViewerHandledObject<T> {
    let inner = Rc::downgrade(&self.inner);
    NyxtViewerHandledObject { handle, inner }
  }
}

pub trait NyxtShadingWrapped: ShadingProvider<GFX> + Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &mut NyxtViewer, handle: ShadingHandle<GFX, Self>) -> Self::Wrapper;
}

pub struct ShadingHandleWrap<T: ShadingProvider<GFX>>(pub ShadingHandle<GFX, T>);
impl<T: ShadingProvider<GFX>> Copy for ShadingHandleWrap<T> {}
impl<T: ShadingProvider<GFX>> Clone for ShadingHandleWrap<T> {
  fn clone(&self) -> Self {
    ShadingHandleWrap(self.0.clone())
  }
}

impl<T: ShadingProvider<GFX>> NyxtViewerHandle for ShadingHandleWrap<T> {
  type Item = <T as rendiation_ral::ShadingProvider<GFX>>::Instance;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    &inner.resource.shadings.get_shading(self.0).data
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.shadings.delete_shading(self.0)
  }
}
impl<T: ShadingProvider<GFX>> NyxtViewerMutableHandle for ShadingHandleWrap<T> {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.resource.shadings.update_shading(self.0)
  }
}

pub trait NyxtBindGroupWrapped: BindGroupProvider<GFX> + Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &mut NyxtViewer, handle: BindGroupHandle<GFX, Self>) -> Self::Wrapper;
}

pub struct BindGroupHandleWrap<T: BindGroupProvider<GFX>>(pub BindGroupHandle<GFX, T>);
impl<T: BindGroupProvider<GFX>> Copy for BindGroupHandleWrap<T> {}
impl<T: BindGroupProvider<GFX>> Clone for BindGroupHandleWrap<T> {
  fn clone(&self) -> Self {
    BindGroupHandleWrap(self.0.clone())
  }
}

impl<T: BindGroupProvider<GFX>> NyxtViewerHandle for BindGroupHandleWrap<T> {
  type Item = <T as rendiation_ral::BindGroupProvider<GFX>>::Instance;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.resource.bindgroups.get_bindgroup_unwrap(self.0)
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.bindgroups.delete(self.0)
  }
}
impl<T: BindGroupProvider<GFX>> NyxtViewerMutableHandle for BindGroupHandleWrap<T> {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.resource.bindgroups.update(self.0)
  }
}

pub trait NyxtUBOWrapped: Sized {
  type Wrapper;

  fn to_nyxt_wrapper(viewer: &mut NyxtViewer, handle: UniformHandle<GFX, Self>) -> Self::Wrapper;
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
