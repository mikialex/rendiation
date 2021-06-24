use arena::Arena;

use super::*;

pub trait Model: 'static {
  fn material(&self) -> MaterialHandle;
  fn mesh(&self) -> MeshHandle;
  fn group(&self) -> MeshDrawGroup;
  fn node(&self) -> SceneNodeHandle;
}

pub struct MeshModel {
  pub material: MaterialHandle,
  pub mesh: MeshHandle,
  pub group: MeshDrawGroup,
  pub node: SceneNodeHandle,
}

impl Model for MeshModel {
  fn material(&self) -> MaterialHandle {
    self.material
  }

  fn mesh(&self) -> MeshHandle {
    self.mesh
  }

  fn group(&self) -> MeshDrawGroup {
    self.group
  }

  fn node(&self) -> SceneNodeHandle {
    self.node
  }
}

// impl MeshModel {
//   pub fn new() -> Self {}
// }

#[derive(Debug, Clone, Copy)]
pub enum MeshDrawGroup {
  Full,
  SubMesh(usize),
}

pub struct ModelPassSetupContext<'a, S> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<Box<dyn Mesh>>,
  pub material_ctx: SceneMaterialPassSetupCtx<'a, S>,
}

impl Scene {
  pub fn add_model(&mut self, model: impl Model) -> ModelHandle {
    self.models.insert(Box::new(model))
  }
}
