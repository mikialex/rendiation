use arena::Arena;

use crate::renderer::Renderer;

use super::*;

pub struct Model {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model {
  pub fn update<'a>(&mut self, ctx: &mut ModelPassPrepareContext<'a>, renderer: &Renderer) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    material.update(renderer, &mut ctx.material_ctx, &OriginForward)
  }

  pub fn setup_pass<'a>(&self, pass: &mut wgpu::RenderPass<'a>, ctx: &ModelPassSetupContext<'a>) {
    let material = ctx.materials.get(self.material).unwrap();
    material.setup_pass(pass, todo!(), &OriginForward);
    let mesh = ctx.meshes.get(self.mesh).unwrap();
    mesh.setup_pass(pass);
  }
}

pub struct ModelPassSetupContext<'a> {
  pub materials: &'a Arena<Box<dyn SceneMaterial>>,
  pub meshes: &'a Arena<SceneMesh>,
}

pub struct ModelPassPrepareContext<'a> {
  pub materials: &'a mut Arena<Box<dyn SceneMaterial>>,
  pub meshes: &'a mut Arena<SceneMesh>,
  pub material_ctx: SceneMaterialRenderPrepareCtx<'a>,
}
