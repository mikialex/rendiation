use arena::Arena;

use super::*;

pub struct Model {
  pub(crate) material: MaterialHandle,
  pub(crate) mesh: MeshHandle,
  pub transform: Transformation,
}

pub struct ModelPassSetupContext<'a, S> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<SceneMesh>,
  pub material_ctx: SceneMaterialPassSetupCtx<'a, S>,
}
