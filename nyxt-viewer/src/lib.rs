use std::{cell::RefCell, rc::Rc, rc::Weak};

use rendiation_ral::*;
pub use rendiation_shader_library::fog::*;

use rendiation_scenegraph::*;

use rendiation_webgl::{WebGL, WebGLRenderer};
use space_indexer::{
  bvh::BalanceTree,
  bvh::{test::bvh_build, SAH},
  utils::generate_boxes_in_space,
  utils::TreeBuildOption,
};
use wasm_bindgen::prelude::*;

pub mod geometry;
pub mod scene;
pub mod ubo;

pub type GFX = WebGL;

// #[wasm_bindgen]
// #[derive(Debug, Copy, Clone)]
// pub struct Fog {
//   pub data: f32,
// }

// #[wasm_bindgen]
// pub struct FogWASM {
//   index: usize,
//   resource: Weak<RefCell<ResourceManager<GFX>>>,
// }

// #[wasm_bindgen]
// pub struct BlockShadingParamGroupWASM {
//   index: usize,
//   resource: Weak<RefCell<ResourceManager<GFX>>>,
// }

// // create WASMWrappedItem_Fog from WASMScene by default value

// #[wasm_bindgen]
// impl BlockShadingParamGroupWASM {
//   #[wasm_bindgen(setter)]
//   pub fn set_fog(&mut self, fog: &FogWASM) {}
// }

#[wasm_bindgen]
pub struct NyxtViewer {
  inner: Rc<RefCell<NyxtViewerInner>>,
}

pub struct NyxtViewerInner {
  renderer: WebGLRenderer,
  resource: ResourceManager<GFX>,
  scene: Scene<GFX>,
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
  handle: Handle,
  inner: Weak<RefCell<NyxtViewerInner>>,
}

impl<Handle: NyxtViewerHandle> NyxtViewerHandledObject<Handle> {
  pub fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut NyxtViewerInner) -> T) -> T {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    let mut inner = inner.borrow_mut();
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

  fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut NyxtViewerInner) -> T) -> T {
    let mut inner = self.inner.borrow_mut();
    mutator(&mut inner)
  }
}

#[wasm_bindgen]
pub fn test_bvh() {
  let boxes = generate_boxes_in_space(20000, 10000., 1.);

  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut BalanceTree,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }

  let mut sah = SAH::new(4);
  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut sah,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }
}

pub use rendiation_shader_library::fog::*;
