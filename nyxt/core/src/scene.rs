use rendiation_math::wasm::{Mat4F32WASM, WASMAbleType};
use rendiation_scenegraph::{
  default_impl::RenderMatrixData, default_impl::SceneNodeData, DrawcallHandle, SceneNodeHandle,
};
use wasm_bindgen::prelude::*;

use crate::{NyxtUBOWrapped, NyxtViewer, NyxtViewerHandledObject, GFX};

#[wasm_bindgen]
pub struct SceneNodeWASM {
  #[wasm_bindgen(skip)]
  pub inner: NyxtViewerHandledObject<SceneNodeHandle<GFX>>,
}

#[wasm_bindgen]
impl SceneNodeWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &NyxtViewer) -> SceneNodeWASM {
    let handle =
      viewer.mutate_inner(|inner| inner.scene.create_new_node(&mut inner.resource).handle());
    Self {
      inner: viewer.make_handle_object(handle),
    }
  }

  // pub fn get_node_matrix_render_info_uniform(
  //   &self,
  // ) -> <RenderMatrixData as NyxtUBOWrapped>::Wrapper {
  //   let handle = self.inner.mutate_item(|d| d.render_data.matrix_data);
  //   RenderMatrixData::to_nyxt_wrapper()
  // }

  #[wasm_bindgen(getter)]
  pub fn local_matrix(&self) -> Mat4F32WASM {
    self.inner.mutate_item(|d| d.local_matrix).to_wasm()
  }

  #[wasm_bindgen(setter)]
  pub fn set_local_matrix(&mut self, value: &Mat4F32WASM) {
    self
      .inner
      .mutate_item(|d| d.local_matrix = WASMAbleType::from_wasm(*value))
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
      .mutate_item(|n| n.drawcalls.push(drawcall.inner.handle))
  }

  #[wasm_bindgen]
  pub fn clear_drawcall(&mut self) {
    self.inner.mutate_item(|n| n.drawcalls.clear())
  }

  #[wasm_bindgen]
  pub fn add_child(&mut self, child: &SceneNodeWASM) {
    self.inner.mutate_inner(|inner| {
      inner
        .scene
        .node_add_child_by_handle(self.inner.handle, child.inner.handle)
    })
  }

  #[wasm_bindgen]
  pub fn remove_child(&mut self, child: &SceneNodeWASM) {
    self.inner.mutate_inner(|inner| {
      inner
        .scene
        .node_remove_child_by_handle(self.inner.handle, child.inner.handle)
    })
  }
}

#[wasm_bindgen]
pub struct DrawcallWASM {
  inner: NyxtViewerHandledObject<DrawcallHandle<GFX>>,
}

#[wasm_bindgen]
impl DrawcallWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(_viewer: &NyxtViewer) -> DrawcallWASM {
    todo!()
    // let handle = viewer.mutate_inner(|inner| inner.scene.create_new_node().handle());
    // Self {
    //   inner: viewer.make_handle_object(handle),
    // }
  }
}
