use std::cell::RefCell;

use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_webgpu::GPURenderPass;

use super::*;

pub struct MeshModel {
  pub material: Rc<RefCell<Box<dyn Material>>>,
  pub mesh: Rc<RefCell<Box<dyn Mesh>>>,
  pub group: MeshDrawGroup,
  pub node: SceneNodeHandle,
}

impl SceneRenderable for MeshModel {
  fn update(
    &mut self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    components: &mut SceneComponents,
  ) {
    let mut material = self.material.borrow_mut();
    let mut mesh = self.mesh.borrow_mut();
    let node = components.nodes.get_node_mut(self.node).data_mut();

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      model_info: node.get_model_gpu(gpu).into(),
      active_mesh: mesh.as_ref().into(),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    components: &SceneComponents,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let material = self.material.borrow();
    let mesh = self.mesh.borrow();
    let m: &Box<dyn Mesh> = &mesh;
    let node = components.nodes.get_node(self.node).data();

    let ctx = SceneMaterialPassSetupCtx {
      pass: pass_info,
      camera_gpu,
      model_gpu: node.gpu.as_ref().unwrap().into(),
      resources,
      active_mesh: Some(m.as_ref()),
    };
    material.setup_pass(pass, &ctx);

    mesh.setup_pass_and_draw(pass, self.group);
  }
}
