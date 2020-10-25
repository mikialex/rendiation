use std::{cell::RefCell, rc::Rc, rc::Weak};

use arena::Handle;
use rendiation_math::wasm::Mat4F32WASM;
use rendiation_scenegraph::{default_impl::SceneNodeData, DrawcallHandle, Scene, SceneNodeHandle};
use wasm_bindgen::prelude::*;

use crate::{geometry::WASMGeometry, NyxtViewer, GFX};

#[wasm_bindgen]
pub struct SceneNodeDataWASM {
  handle: SceneNodeHandle<GFX>,
  scene: Weak<RefCell<Scene<GFX>>>,
}

impl SceneNodeDataWASM {
  fn mutate<T>(&self, mutator: impl FnOnce(&mut SceneNodeData<GFX>) -> T) -> T {
    mutator(
      Weak::upgrade(&self.scene)
        .unwrap()
        .borrow_mut()
        .get_node_mut(self.handle)
        .data_mut(),
    )
  }
}

#[wasm_bindgen]
impl SceneNodeDataWASM {
  #[wasm_bindgen(getter)]
  pub fn get_local_matrix(&self) -> Mat4F32WASM {
    bytemuck::cast(self.mutate(|d| d.local_matrix))
  }

  #[wasm_bindgen(setter)]
  pub fn set_local_matrix(&mut self, value: Mat4F32WASM) {
    self.mutate(|d| d.local_matrix = bytemuck::cast(value))
  }

  pub fn get_visible(&self) -> bool {
    self.mutate(|d| d.visible)
  }

  #[wasm_bindgen(setter)]
  pub fn set_visible(&mut self, value: bool) {
    self.mutate(|d| d.visible = value)
  }

  pub fn push_drawcall(&mut self, drawcall: &DrawcallWASM) {
    self.mutate(|n| n.drawcalls.push(drawcall.handle))
  }

  pub fn clear_drawcall(&mut self) {
    self.mutate(|n| n.drawcalls.clear())
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

impl Drop for SceneNodeDataWASM {
  fn drop(&mut self) {
    todo!()
    // let handle = self.mutate_resource(|r| r.delete_index_buffer(index_buffer));
  }
}

#[wasm_bindgen]
pub struct DrawcallWASM {
  handle: DrawcallHandle<GFX>,
  scene: Weak<RefCell<Scene<GFX>>>,
}

#[wasm_bindgen]
impl NyxtViewer {
  pub fn create_drawcall(
    &self,
    geometry: WASMGeometry,
    shading: *const Handle<usize>, // todo
  ) -> DrawcallWASM {
    todo!()
    // let mut scene = self.scene.borrow_mut();
    // let node = scene.create_drawcall();
    // DrawcallWASM {
    //   handle: node.handle(),
    //   scene: Rc::downgrade(&self.scene),
    // }
  }
}

impl Drop for DrawcallWASM {
  fn drop(&mut self) {
    todo!()
    // let handle = self.mutate_resource(|r| r.delete_index_buffer(index_buffer));
  }
}
