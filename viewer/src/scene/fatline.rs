use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_webgpu::{GPURenderPass, GPU};

use crate::*;

pub struct FatlineImpl {
  pub material: MaterialCell<FatLineMaterial>,
  pub mesh: FatlineMeshCellImpl,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl FatlineImpl {
  pub fn new(
    material: MaterialCell<FatLineMaterial>,
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

    self.node.mutate(|node| {
      let mut ctx = SceneMaterialRenderPrepareCtx {
        base,
        model_info: node.get_model_gpu(gpu).into(),
        active_mesh: Some(mesh),
      };

      material.update(gpu, &mut ctx);

      mesh.update(gpu, &mut base.resources.custom_storage);
    });
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    self.node.visit(|node| {
      let ctx = SceneMaterialPassSetupCtx {
        pass: pass_info,
        camera_gpu,
        model_gpu: node.gpu.as_ref().unwrap().into(),
        resources,
        active_mesh: Some(mesh),
      };
      material.setup_pass(pass, &ctx);

      mesh.setup_pass_and_draw(pass, self.group);
    });
  }
}
