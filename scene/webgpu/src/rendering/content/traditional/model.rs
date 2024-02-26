use crate::*;

pub struct StandardModelGPUResource {
  models: StorageReadView<StandardModel>,
  mat: MaterialsGPUResource,
  mesh: MeshGPUResource,
}

impl StandardModelGPUResource {
  pub fn prepare_render(
    &self,
    m: AllocIdx<StandardModel>,
  ) -> (SceneMaterialRenderComponent, SceneMeshRenderComponent) {
    let model = self.models.get(m).unwrap();
    let mat = self.mat.prepare_render(&model.material);
    let mesh = self.mesh.prepare_render(&model.mesh);
    (mat, mesh)
  }
}
