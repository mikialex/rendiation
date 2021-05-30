use arena::Arena;

use crate::renderer::Renderer;

use super::*;

pub struct Model {
  pub(crate) material: MaterialHandle,
  pub(crate) mesh: MeshHandle,
}

impl Model {
  pub fn setup_pass<'a, S: RenderStyle>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &ModelPassSetupContext<'a, S>,
  ) {
    let material = ctx.materials.get(self.material).unwrap().as_ref();
    S::setup_pass(material, pass, &ctx.material_ctx);
    let mesh = ctx.meshes.get(self.mesh).unwrap();
    mesh.setup_pass(pass);
  }
}

pub struct ModelPassSetupContext<'a, S> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<SceneMesh>,
  pub material_ctx: SceneMaterialPassSetupCtx<'a, S>,
}
