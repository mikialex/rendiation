use super::scene::Scene;
use generational_arena::Index;
use rendiation_math::{Mat4, One};
use rendiation_render_entity::BoundingData;

pub struct SceneNode {
  self_id: Index,
  parent: Option<Index>,
  children: Vec<Index>,
}

impl SceneNode {
  pub(crate) fn new() -> Self {
    Self {
      self_id: Index::from_raw_parts(0, 0), // later
      parent: None,
      children: Vec::new(),
    }
  }

  pub(crate) fn set_self_id(&mut self, id: Index) -> &mut Self {
    self.self_id = id;
    self
  }

  pub fn traverse(&self, scene: &mut Scene, mut visitor: impl FnMut(&SceneNode)) {
    let mut visit_stack: Vec<Index> = Vec::new();
    visit_stack.push(self.self_id);

    while let Some(index) = visit_stack.pop() {
      let node_to_visit = scene.get_node(index);
      visitor(node_to_visit);
      visit_stack.extend(node_to_visit.children.iter().cloned())
    }
  }

  pub fn add(&mut self, child_to_add: &mut SceneNode) -> &mut Self {
    if child_to_add.parent.is_some() {
      panic!("child node already has a parent");
    }
    child_to_add.parent = Some(self.self_id);
    self.children.push(child_to_add.self_id);
    self
  }

  pub fn remove(&mut self, child_to_remove: &mut SceneNode) -> &mut Self {
    let child_index = self
      .children
      .iter()
      .position(|&x| x == child_to_remove.self_id)
      .expect("tried to remove nonexistent child");

    self.children.swap_remove(child_index);
    self
  }
}

pub trait TransformLocalWorld {
  fn get_local_transform();
  fn set_local_transform();
  fn get_world_transform();
  fn set_world_transform();
}

pub struct RenderObject {
  shading_index: Index,
  geometry_index: Index,
}

pub struct RenderData {
  world_bounding: Option<BoundingData>,
  world_matrix: Mat4<f32>,
  local_matrix: Mat4<f32>,
  normal_matrix: Mat4<f32>,
}

impl RenderData {
  pub fn new() -> Self {
    Self {
      world_bounding: None,
      world_matrix: Mat4::one(),
      local_matrix: Mat4::one(),
      normal_matrix: Mat4::one(),
    }
  }
}

pub struct ResourceManager {
  // shadings
// geometries
}
