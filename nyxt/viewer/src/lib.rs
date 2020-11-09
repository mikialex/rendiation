pub use nyxt_core::*;

use rendiation_math::Vec4;
use rendiation_mesh_buffer::vertex::Vertex;
use rendiation_ral::{ShaderWithGeometry, RAL};
use space_indexer::{
  bvh::BalanceTree,
  bvh::{test::bvh_build, SAH},
  utils::generate_boxes_in_space,
  utils::TreeBuildOption,
};
use wasm_bindgen::prelude::*;

use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;

pub mod test;

pub mod scene;
pub mod viewer_impl;
pub use scene::*;
pub use viewer_impl::*;
pub mod geometry;
pub use geometry::*;

#[wasm_bindgen]
pub struct NyxtViewer {
  inner: Rc<RefCell<NyxtViewerInner>>,
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

  pub fn make_handle_object<V: NyxtViewerInnerTrait, T: NyxtViewerHandle<V>>(
    &self,
    handle: T,
  ) -> NyxtViewerHandledObject<V, T> {
    let inner = Rc::downgrade(&self.inner);
    NyxtViewerHandledObject { handle, inner }
  }
}

impl<V: NyxtViewerInnerTrait> NyxtViewerHandle<V> for DrawcallHandle<GFX> {
  type Item = Drawcall<GFX>;

  fn get(self, inner: &V) -> &Self::Item {
    inner.scene.drawcalls.get(self).unwrap()
  }
  fn free(self, inner: &mut V) {
    inner.scene.drawcalls.remove(self);
  }
}

impl<V: NyxtViewerInnerTrait> NyxtViewerHandle<V> for SceneNodeHandle<GFX> {
  type Item = SceneNodeData<GFX>;

  fn get(self, inner: &V) -> &Self::Item {
    inner.scene.get_node(self).data()
  }
  fn free(self, inner: &mut V) {
    inner.scene.free_node(self)
  }
}
impl<V: NyxtViewerInnerTrait> NyxtViewerMutableHandle<V> for SceneNodeHandle<GFX> {
  fn get_mut(self, inner: &mut V) -> &mut Self::Item {
    inner.scene.get_node_mut(self).data_mut()
  }
}
