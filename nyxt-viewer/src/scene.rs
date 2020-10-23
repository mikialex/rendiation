use std::{cell::RefCell, rc::Rc, rc::Weak};

use rendiation_math::wasm::Mat4F32WASM;
use rendiation_scenegraph::{data, default_impl::SceneNodeData, Scene, SceneNodeHandle};
use wasm_bindgen::prelude::*;

use crate::{NyxtViewer, GFX};

#[wasm_bindgen]
pub struct SceneNodeDataWASM {
  handle: SceneNodeHandle<GFX>,
  scene: Weak<RefCell<Scene<GFX>>>,
}

impl SceneNodeDataWASM {
  fn mutate<T>(&self, mutator: impl FnOnce(SceneNodeData<GFX>) -> T) -> T {
    // mutator(self.scene.borrow_mut())
    todo!()
  }
}

#[wasm_bindgen]
impl SceneNodeDataWASM {
  #[wasm_bindgen(getter)]
  pub fn get_local_matrix(&self) -> Mat4F32WASM {
    todo!()
  }

  #[wasm_bindgen(setter)]
  pub fn set_local_matrix(&mut self, value: Mat4F32WASM) {
    todo!()
  }
}

#[wasm_bindgen]
impl NyxtViewer {
  pub fn create_node(&self) -> SceneNodeDataWASM {
    let mut scene = self.scene.borrow_mut();
    let node = scene.create_new_node();
    SceneNodeDataWASM {
      handle: node.handle(),
      scene: Rc::downgrade(&self.scene),
    }
  }
}
