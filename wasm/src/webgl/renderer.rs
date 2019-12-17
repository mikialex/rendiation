use rendiation_math::*;
use crate::scene_graph::*;
use crate::webgl::buffer_attribute::*;
use rendiation_render_entity::*;
use core::cell::{Cell, RefCell};
use fnv::FnvHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::*;

use crate::{log, log_f32, log_usize};

#[wasm_bindgen(raw_module = "../src/webgl/upload-util")]
extern "C" {
  pub fn makeBuffer(size: usize) -> JsValue;
  pub fn copyBuffer(buffer: &JsValue, start: *const f32, offset: usize);
  pub fn uploadMatrix4f(
    gl: &WebGlRenderingContext,
    location: &WebGlUniformLocation,
    buffer: &JsValue,
  );
}

type ProgramMap = RefCell<
  HashMap<
    Rc<dyn Shading<WebGLRenderer>>,
    Rc<dyn ShadingGPUPort<WebGLRenderer>>,
    BuildHasherDefault<FnvHasher>,
  >,
>;

#[wasm_bindgen]
pub struct WebGLRenderer {
  pub(crate) step_id: Cell<usize>,
  pub(crate) gl: Rc<WebGlRenderingContext>,

  pub(crate) model_transform: JsValue,
  pub(crate) camera_projection: JsValue,
  pub(crate) camera_inverse: JsValue,

  pub(crate) active_port: Option<Rc<dyn ShadingGPUPort<Self>>>,

  pub(crate) programs: ProgramMap,
  pub(crate) buffer_manager: BufferManager,
}

#[wasm_bindgen]
impl WebGLRenderer {
  pub fn new(canvas: HtmlCanvasElement) -> Result<WebGLRenderer, JsValue> {
    let context = canvas
      .get_context("webgl")?
      .unwrap()
      .dyn_into::<WebGlRenderingContext>()?;

    context.enable(WebGlRenderingContext::DEPTH_TEST);
    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.get_extension("OES_element_index_uint")?;

    let gl = Rc::new(context);
    let glc = gl.clone();
    Ok(WebGLRenderer {
      step_id: Cell::new(0),
      gl,
      model_transform: makeBuffer(16),
      camera_projection: makeBuffer(16),
      camera_inverse: makeBuffer(16),
      active_port: None,
      programs: RefCell::new(HashMap::with_hasher(
        BuildHasherDefault::<FnvHasher>::default(),
      )),
      buffer_manager: BufferManager::new(glc),
    })
  }

  pub fn render(&mut self, scene: &SceneGraph) {
    self.gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    self.gl.clear(WebGlRenderingContext::DEPTH_BUFFER_BIT);

    let list = scene.batch_drawcalls().borrow();

    copyBuffer(
      &self.camera_projection,
      scene.camera.projection_matrix.as_ptr(),
      16,
    );
    copyBuffer(
      &self.camera_inverse,
      scene.camera.inverse_world_matrix.as_ptr(),
      16,
    );
    list.foreach(|render_item| {
      let object = scene
        .store
        .render_objects
        .get(render_item.render_object_index);
      let scene_node = scene.nodes.get(render_item.scene_node_index).borrow();

      copyBuffer(&self.model_transform, scene_node.matrix_world.as_ptr(), 16);

      let port = self.get_port(object.shading.clone()).unwrap();

      if let Some(active_port) = &self.active_port {
        if *active_port != port.clone() {
          port.use_self(self);
        }
      } else {
        port.use_self(self);
      }

      port.use_uniforms(self);
      port.use_geometry(self, object.geometry.clone());
      self.draw(object.geometry.clone());
    })
  }
}

impl WebGLRenderer {
  pub fn draw(&mut self, geometry: Rc<dyn Geometry>) {
    let length = geometry.get_draw_count_all();
    if geometry.is_index_draw() {
      self.gl.draw_elements_with_i32(
        WebGlRenderingContext::TRIANGLES,
        0,
        WebGlRenderingContext::UNSIGNED_INT,
        length as i32,
      );
    } else {
      self
        .gl
        .draw_arrays(WebGlRenderingContext::TRIANGLES, 0, length as i32);
    }
  }
}
