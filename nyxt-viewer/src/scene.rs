use std::{cell::RefCell, rc::Weak};

use arena::Handle;
use rendiation_math::wasm::Mat4F32WASM;
use rendiation_scenegraph::{default_impl::SceneNodeData, DrawcallHandle, Scene, SceneNodeHandle};
use wasm_bindgen::prelude::*;

use crate::{
  geometry::WASMGeometry, NyxtViewer, NyxtViewerHandle, NyxtViewerHandledObject, NyxtViewerInner,
  NyxtViewerMutableHandle, GFX,
};

#[wasm_bindgen]
pub struct SceneNodeDataWASM {
  inner: NyxtViewerHandledObject<SceneNodeHandleWrap>,
}

#[derive(Copy, Clone)]
pub struct SceneNodeHandleWrap(SceneNodeHandle<GFX>);
impl NyxtViewerHandle for SceneNodeHandleWrap {
  type Item = SceneNodeData<GFX>;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.scene.get_node(self.0).data()
  }
  fn free(self, _inner: &mut NyxtViewerInner) {
    todo!()
  }
}
impl NyxtViewerMutableHandle for SceneNodeHandleWrap {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.scene.get_node_mut(self.0).data_mut()
  }
}

#[wasm_bindgen]
impl SceneNodeDataWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &NyxtViewer) -> SceneNodeDataWASM {
    let handle = viewer.mutate_inner(|inner| inner.scene.create_new_node().handle());
    Self {
      inner: viewer.make_handle_object(SceneNodeHandleWrap(handle)),
    }
  }

  #[wasm_bindgen(getter)]
  pub fn get_local_matrix(&self) -> Mat4F32WASM {
    bytemuck::cast(self.inner.mutate_item(|d| d.local_matrix))
  }

  #[wasm_bindgen(setter)]
  pub fn set_local_matrix(&mut self, value: Mat4F32WASM) {
    self
      .inner
      .mutate_item(|d| d.local_matrix = bytemuck::cast(value))
  }

  pub fn get_visible(&self) -> bool {
    self.inner.mutate_item(|d| d.visible)
  }

  #[wasm_bindgen(setter)]
  pub fn set_visible(&mut self, value: bool) {
    self.inner.mutate_item(|d| d.visible = value)
  }

  #[wasm_bindgen]
  pub fn push_drawcall(&mut self, drawcall: &DrawcallWASM) {
    self
      .inner
      .mutate_item(|n| n.drawcalls.push(drawcall.handle))
  }

  #[wasm_bindgen]
  pub fn clear_drawcall(&mut self) {
    self.inner.mutate_item(|n| n.drawcalls.clear())
  }

  #[wasm_bindgen]
  pub fn add_child(&mut self, child: &SceneNodeDataWASM) {
    self.inner.mutate_inner(|inner| {
      inner
        .scene
        .node_add_child_by_handle(self.inner.handle.0, child.inner.handle.0)
    })
  }

  #[wasm_bindgen]
  pub fn remove_child(&mut self, child: &SceneNodeDataWASM) {
    self.inner.mutate_inner(|inner| {
      inner
        .scene
        .node_remove_child_by_handle(self.inner.handle.0, child.inner.handle.0)
    })
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
