use crate::webgl::renderer::WebGLRenderer;
use rendiation_math_entity::frustum::Frustum;
use rendiation_render_entity::*;
use rendiation_math::*;
use crate::scene_graph::*;
use crate::utils::set_panic_hook;
use crate::utils::ArrayContainer;
use core::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

pub struct RenderEntityStore<Renderer>{
  pub(crate) buffers: ArrayContainer<Rc<BufferData<f32>>>,
  pub(crate) index_buffers: ArrayContainer<Rc<BufferData<u16>>>,
  pub(crate) geometries: ArrayContainer<Rc<dyn Geometry>>,
  pub(crate) shadings: ArrayContainer<Rc<dyn Shading<Renderer>>>,
  pub(crate) render_objects: ArrayContainer<RenderObject<Renderer>>,
}

impl<Renderer> RenderEntityStore<Renderer>{
  fn new() -> Self {
    RenderEntityStore {
      buffers: ArrayContainer::new(),
      index_buffers: ArrayContainer::new(),
      geometries: ArrayContainer::new(),
      shadings: ArrayContainer::new(),
      render_objects: ArrayContainer::new(),
    }
  }
}

#[wasm_bindgen]
pub struct SceneGraph {
  pub(crate) camera: AnyCamera,
  camera_frustum: Frustum,
  pub(crate) nodes: ArrayContainer<RefCell<SceneNode>>,

  pub(crate) store: RenderEntityStore<WebGLRenderer>,

  render_list: RefCell<RenderList>,
}

impl SceneGraph {
  pub fn set_camera(&mut self, camera: AnyCamera) {
    self.camera = camera;
    self.camera_frustum.set_from_matrix(
      self.camera.projection_matrix * self.camera.inverse_world_matrix
    );
  }

  pub fn get_scene_node(&self, index: usize) -> &RefCell<SceneNode> {
    self.nodes.get(index)
  }

  pub fn traverse<T>(&self, node: &RefCell<SceneNode>, mut visitor: T)
  where
    T: FnMut(&RefCell<SceneNode>, &SceneGraph) -> (),
  {
    let mut traverse_stack: Vec<&RefCell<SceneNode>> = Vec::new();
    traverse_stack.push(node);

    while let Some(node_to_visit) = traverse_stack.pop() {
      visitor(node_to_visit, self);

      // add children to stack
      if let Some(first_child_index) = node_to_visit.borrow().first_child {
        let first_child = self.get_scene_node(first_child_index);
        traverse_stack.push(first_child);
        let mut child_next = first_child;
        while let Some(next_child_index) = child_next.borrow().right_brother {
          let next_child = self.get_scene_node(next_child_index);
          traverse_stack.push(next_child);
          child_next = next_child
        }
      }
    }
  }

  pub fn batch_drawcalls(&self) -> &RefCell<RenderList> {
    let root = self.get_scene_node(0);
    let mut render_list = self.render_list.borrow_mut();
    render_list.reset();
    let project_screen_matrix = self.camera.projection_matrix * self.camera.inverse_world_matrix;

    self.traverse(root, |node: &RefCell<SceneNode>, scene: &SceneGraph| {
      let mut self_node = node.borrow_mut();

      if let Some(parent_index) = self_node.parent {
        let parent_node = scene.get_scene_node(parent_index).borrow_mut();

        self_node.matrix_local = compose(&self_node.position, &self_node.rotation, &self_node.scale);
        
        self_node.matrix_world = parent_node.matrix_world * self_node.matrix_local;

        // if let Some(render_object_index) = self_node.render_data {
        //   let render_object = self.render_objects.get(render_object_index);
        //   render_object.update_world_bounding(&self_node.matrix_world);
        //   if self.camera_frustum.intersects_sphere(&render_object.world_bounding_sphere) {
        //     // calculate distance to camera for sorting;
        //     let z =  (self_node.matrix_world.position() * project_screen_matrix).z;
        //     render_list.add_renderable(render_object, &self_node, z);
        //   }
        // }

        if let Some(render_object_index) = self_node.render_data {
          let render_object = self.store.render_objects.get(render_object_index);
          let z =  (self_node.matrix_world.position() * project_screen_matrix).z;
          render_list.add_renderable(render_object, &self_node, z);
        }

      } else {
        self_node.matrix_local =
          compose(&self_node.position, &self_node.rotation, &self_node.scale);
        self_node.matrix_world =  self_node.matrix_local;
      }
    });

    render_list.sort();
    &self.render_list
  }
}

fn compose(position: &Vec3<f32>, quaternion: &Quat<f32>, scale: &Vec3<f32>) -> Mat4<f32> {
  let x = quaternion.x;
  let y = quaternion.y;
  let z = quaternion.z;
  let w = quaternion.w;
  let x2 = x + x;
  let y2 = y + y;
  let z2 = z + z;
  let xx = x * x2;
  let xy = x * y2;
  let xz = x * z2;
  let yy = y * y2;
  let yz = y * z2;
  let zz = z * z2;
  let wx = w * x2;
  let wy = w * y2;
  let wz = w * z2;

  let sx = scale.x;
  let sy = scale.y;
  let sz = scale.z;

  Mat4::new(
    (1. - (yy + zz)) * sx,
    (xy + wz) * sx,
    (xz - wy) * sx,
    0.,
    (xy - wz) * sy,
    (1. - (xx + zz)) * sy,
    (yz + wx) * sy,
    0.,
    (xz + wy) * sz,
    (yz - wx) * sz,
    (1. - (xx + yy)) * sz,
    0.,
    position.x,
    position.y,
    position.z,
    1.,
  )
}

#[wasm_bindgen]
impl SceneGraph {
  pub fn new() -> SceneGraph {
    set_panic_hook();
    let mut graph = SceneGraph {
      camera: AnyCamera::new(),
      camera_frustum: Frustum::new(),
      nodes: ArrayContainer::new(),
      store: RenderEntityStore::new(),
      render_list: RefCell::new(RenderList::new()),
    };
    graph.create_new_node(); // as root
    graph
  }

  pub fn update_all_world_matrix(&mut self) {
    let root = self.get_scene_node(0);
    self.traverse(root, |node: &RefCell<SceneNode>, scene: &SceneGraph| {
      let mut self_node = node.borrow_mut();
      if let Some(parent_index) = self_node.parent {
        let parent_node = scene.get_scene_node(parent_index).borrow_mut();
        self_node.matrix_world = self_node.matrix_world * parent_node.matrix_world;
      }
    })
  }
}
