use super::scene::Scene;
use generational_arena::Index;
use rendiation_math::{Mat4, One};
use rendiation_render_entity::BoundingData;

pub struct SceneNode {
  pub(crate) self_id: Index,
  pub(crate) parent: Option<Index>,
  pub(crate) children: Vec<Index>,
  pub render_objects: Vec<Index>,
  pub(crate) render_data: RenderData,
}

impl SceneNode {
  pub(crate) fn new() -> Self {
    Self {
      self_id: Index::from_raw_parts(0, 0), // later
      parent: None,
      children: Vec::new(),
      render_objects: Vec::new(),
      render_data: RenderData::new()
    }
  }

  pub(crate) fn set_self_id(&mut self, id: Index) -> &mut Self {
    self.self_id = id;
    self
  }

  pub fn get_id(&self) -> Index {
    self.self_id
  }

  pub fn add_render_object(&mut self, id: Index) {
    self.render_objects.push(id)
  }

  pub fn traverse(&self, scene: &Scene, mut visitor: impl FnMut(&SceneNode)) {
    let mut visit_stack: Vec<Index> = Vec::new(); // TODO reuse
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

pub struct RenderData {
  pub world_bounding: Option<BoundingData>,
  pub world_matrix: Mat4<f32>,
  pub local_matrix: Mat4<f32>,
  pub normal_matrix: Mat4<f32>,
  pub camera_distance: f32,
}

impl RenderData {
  pub fn new() -> Self {
    Self {
      world_bounding: None,
      world_matrix: Mat4::one(),
      local_matrix: Mat4::one(),
      normal_matrix: Mat4::one(),
      camera_distance: 0.,
    }
  }
}
