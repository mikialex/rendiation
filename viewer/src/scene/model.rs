use arena::Arena;

use crate::renderer::Renderer;

use super::*;

pub struct Model {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model {
  pub fn update<'a, S: RenderStyle>(
    &mut self,
    ctx: &mut ModelPassPrepareContext<'a, S>,
    renderer: &Renderer,
  ) {
    let material = ctx.materials.get_mut(self.material).unwrap().as_mut();
    S::update(material, renderer, &mut ctx.material_ctx);
  }

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

pub struct ModelPassPrepareContext<'a, S> {
  pub materials: &'a mut Arena<Box<dyn Material>>,
  pub meshes: &'a mut Arena<SceneMesh>,
  pub material_ctx: SceneMaterialRenderPrepareCtx<'a, S>,
}
