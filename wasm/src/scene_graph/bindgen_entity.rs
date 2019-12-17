use crate::scene_graph::*;
use rendiation_render_entity::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl SceneGraph {
  pub fn update_camera(&mut self, proj: &[f32], inverse: &[f32]) {
    self.camera.update_projection(proj);
    self.camera.update_inverse(inverse);
  }

  // pub fn create_new_shading(&mut self, vertex_str: String, frag_str: String) -> usize {
  //   let free_index = self.shadings.get_free_index();
  //   let shading = Rc::new(Shading::new(free_index, vertex_str, frag_str));
  //   self.shadings.set_item(shading, free_index);
  //   free_index
  // }

  pub fn create_new_shading(&mut self, shading_key: String) -> usize {
    if shading_key == "test" {
      let free_index = self.store.shadings.get_free_index();
      // let shading = Rc::new(DynamicShading::new(
      //   free_index,
      //   String::from(
      //     r#"           
      //       attribute vec3 position;
      //       uniform mat4 model_matrix;
      //       uniform mat4 camera_inverse;
      //       uniform mat4 projection_matrix;
      //       void main() {
      //         gl_Position = projection_matrix * camera_inverse * model_matrix * vec4(position, 1.0);
      //       }
      //       "#,
      //   ),
      //   String::from(
      //     r#"
      //       void main() {
      //           gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
      //       }
      //   "#,
      //   ),
      //   vec![String::from("position")],
      //   vec![String::from("model_matrix"), String::from("projection_matrix"), String::from("camera_inverse")],
      // ));
      let shading = Rc::new(PureColorShading::new(free_index));
      self.store.shadings.set_item(shading, free_index);
      free_index
    } else {
      panic!("not supported shading")
    }
  }

  pub fn create_new_buffer_data(&mut self, data: Vec<f32>, stride: usize) -> usize {
    let free_index = self.store.buffers.get_free_index();
    let buffer_data = Rc::new(BufferData::new(free_index, data, stride));
    self.store.buffers.set_item(buffer_data, free_index);
    free_index
  }

  pub fn create_new_index_buffer_data(&mut self, data: Vec<u16>, stride: usize) -> usize {
    let free_index = self.store.index_buffers.get_free_index();
    let buffer_data = Rc::new(BufferData::new(free_index, data, stride));
    self.store.index_buffers.set_item(buffer_data, free_index);
    free_index
  }


  pub fn create_geometry(&mut self, index_index: Option<usize>, position_index: usize) -> Result<usize, JsValue> {
    let free_index = self.store.geometries.get_free_index();
    let position = self.store.buffers.get(position_index);
    let index = if let Some(i) = index_index {
      Some(self.store.index_buffers.get(i).clone())
    } else {
      None
    };
    let geometry = Rc::new(DynamicGeometry::new(free_index, index, position.clone())?);
    self.store.geometries.set_item(geometry, free_index);
    Ok(free_index)
  }

  pub fn create_render_data(&mut self, geometry_id: usize, shading_id: usize) -> usize {
    let geometry = self.store.geometries.get(geometry_id).clone();
    let shading = self.store.shadings.get(shading_id).clone();
    let free_index = self.store.render_objects.get_free_index();
    let obj = RenderObject::new(free_index, shading, geometry);
    self.store.render_objects.set_item(obj, free_index);
    free_index
  }

  pub fn set_render_descriptor(&mut self, render_object_id: usize, node_index: usize) {
    self.get_scene_node(node_index).borrow_mut().render_data = Some(render_object_id);
  }
}
