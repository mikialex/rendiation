use super::*;
use arena::Arena;

pub struct Model {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model {
  pub fn update<'a, F: FnMut(&mut dyn Material)>(
    &mut self,
    ctx: &mut ModelPassPrepareContext,
    f: &mut F,
  ) {
    let material = ctx.materials.get_mut(self.material).unwrap().as_mut();
    f(material);
  }

  pub fn setup_pass<'a, S>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &ModelPassSetupContext<'a, S>,
  ) {
    let material = ctx.materials.get(self.material).unwrap();
    material.setup_pass(pass, todo!(), &OriginForward);
    let mesh = ctx.meshes.get(self.mesh).unwrap();
    mesh.setup_pass(pass);
  }
}

pub struct ModelPassSetupContext<'a, S> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<SceneMesh>,
  pub style: &'a S,
}

pub struct ModelPassPrepareContext<'a> {
  pub materials: &'a mut Arena<Box<dyn Material>>,
  pub meshes: &'a mut Arena<SceneMesh>,
}
