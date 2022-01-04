use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_webgpu::GPU;

use crate::*;

pub struct FatlineImpl {
  pub material: MaterialCell<SceneMaterial<FatLineMaterial>>,
  pub mesh: FatlineMeshCellImpl,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl FatlineImpl {
  pub fn new(
    material: MaterialCell<SceneMaterial<FatLineMaterial>>,
    mesh: FatlineMeshCellImpl,
    node: SceneNode,
  ) -> Self {
    Self {
      material,
      mesh,
      group: Default::default(),
      node,
    }
  }
}

impl SceneRenderable for FatlineImpl {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let material = &mut self.material;
    let mesh = &mut self.mesh;

    self.node.check_update_gpu(base.resources, gpu);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      active_mesh: Some(mesh),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    self.node.visit(|node| {
      let ctx = SceneMaterialPassSetupCtx {
        camera_gpu,
        model_gpu: resources.nodes.get_unwrap(node).into(),
        resources,
      };
      material.setup_pass(pass, &ctx);

      mesh.setup_pass_and_draw(pass, self.group);
    });
  }
}
