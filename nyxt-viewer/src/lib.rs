use std::{cell::RefCell, rc::Rc, rc::Weak};

use arena::Handle;
use rendiation_math::Mat4;
use rendiation_ral::*;

use rendiation_scenegraph::default_impl::*;
use rendiation_scenegraph::*;

use rendiation_webgl::{WebGL, WebGLRenderer};
use wasm_bindgen::prelude::*;

mod geometry;
mod scene;
pub mod ubo;
use geometry::*;

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
  renderer: WebGLRenderer,
  resource: Rc<RefCell<ResourceManager<GFX>>>,
  scene: Rc<RefCell<Scene<GFX>>>,
}

impl NyxtViewer {
  pub fn make_resource(&self) -> Weak<RefCell<ResourceManager<GFX>>> {
    Rc::downgrade(&self.resource)
  }
}

#[wasm_bindgen]
impl NyxtViewer {
  #[wasm_bindgen(constructor)]
  pub fn new(canvas: HtmlCanvasElement) -> Self {
    console_error_panic_hook::set_once();
    Self {
      renderer: WebGLRenderer::new(canvas),
      resource: Rc::new(RefCell::new(ResourceManager::new())),
      scene: Rc::new(RefCell::new(Scene::new())),
    }
  }

  fn mutate_resource<T>(&self, mutator: impl FnOnce(&mut ResourceManager<GFX>) -> T) -> T {
    let mut resource = self.resource.borrow_mut();
    mutator(&mut resource)
  }

  // #[wasm_bindgen]
  // pub fn node_add_child_by_handle(&mut self, parent_handle: usize, child_handle: usize) {
  //   self.scene.node_add_child_by_handle(
  //     self.get_handle(parent_handle).into(),
  //     self.get_handle(child_handle).into(),
  //   );
  // }

  // #[wasm_bindgen]
  // pub fn node_remove_child_by_handle(&mut self, parent_handle: usize, child_handle: usize) {
  //   self.scene.node_remove_child_by_handle(
  //     self.get_handle(parent_handle).into(),
  //     self.get_handle(child_handle).into(),
  //   );
  // }
}

pub enum WebGLAttributeBufferFormat {
  Float,
  Float2,
  Float3,
}
