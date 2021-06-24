use arena::Arena;

use super::*;

pub struct Model {
  pub material: MaterialHandle,
  pub mesh: MeshHandle,
  pub group: MeshDrawGroup,
  pub node: SceneNodeHandle,
}

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
  pub fn add_model(&mut self, model: Model) -> ModelHandle {
    self.models.insert(model)
  }
}
