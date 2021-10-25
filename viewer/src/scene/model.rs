use std::cell::RefCell;

use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_webgpu::GPURenderPass;

use super::*;

#[derive(Clone)]
pub struct MeshModel {
  pub inner: Rc<RefCell<MeshModelInner>>,
}

impl MeshModel {
  // todo add type constraint
  pub fn new<Ma: Material + 'static, Me: Mesh + 'static>(
    material: Ma,
    mesh: Me,
    node: SceneNodeHandle,
  ) -> Self {
    let inner = MeshModelInner {
      material: Box::new(material),
      mesh: Box::new(mesh),
      group: Default::default(),
      node,
    };
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

impl SceneRenderable for MeshModel {
  fn update(
    &mut self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    components: &mut SceneComponents,
  ) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, base, components)
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    components: &SceneComponents,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let inner = self.inner.borrow();
    inner.setup_pass(pass, components, camera_gpu, resources, pass_info)
  }
}

pub struct MeshModelInner {
  pub material: Box<dyn Material>,
  pub mesh: Box<dyn Mesh>,
  pub group: MeshDrawGroup,
  pub node: SceneNodeHandle,
}

impl SceneRenderable for MeshModelInner {
  fn update(
    &mut self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    components: &mut SceneComponents,
  ) {
    let material = &mut self.material;
    let mesh = &mut self.mesh;
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
    let material = &self.material;
    let mesh = &self.mesh;
    let node = components.nodes.get_node(self.node).data();

    let ctx = SceneMaterialPassSetupCtx {
      pass: pass_info,
      camera_gpu,
      model_gpu: node.gpu.as_ref().unwrap().into(),
      resources,
      active_mesh: mesh.as_ref().into(),
    };
    material.setup_pass(pass, &ctx);

    mesh.setup_pass_and_draw(pass, self.group);
  }
}
