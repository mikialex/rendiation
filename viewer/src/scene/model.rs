use std::marker::PhantomData;

use rendiation_renderable_mesh::group::MeshDrawGroup;

use super::*;

pub trait Model {
  fn material(&self) -> MaterialHandle;
  fn mesh(&self) -> MeshHandle;
  fn group(&self) -> MeshDrawGroup;
  fn node(&self) -> SceneNodeHandle;
}

pub struct TypedHandle<T, H> {
  pub(crate) handle: H,
  pub(crate) ty: PhantomData<T>,
}

impl<T, H: Clone> Clone for TypedHandle<T, H> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle.clone(),
      ty: PhantomData,
    }
  }
}

impl<T, H: Copy> Copy for TypedHandle<T, H> {}

pub type TypedMaterialHandle<T> = TypedHandle<T, MaterialHandle>;
pub type TypedMeshHandle<T> = TypedHandle<T, MeshHandle>;

impl SceneRenderable for dyn Model {
  fn update(
    &mut self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    components: &mut SceneComponents,
  ) {
    let material = components
      .materials
      .get_mut(self.material())
      .unwrap()
      .as_mut();
    let mesh = components.meshes.get_mut(self.mesh()).unwrap();
    let node = components.nodes.get_node_mut(self.node()).data_mut();

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      model_info: node.get_model_gpu(gpu).into(),
      active_mesh: mesh.as_ref().into(),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    components: &'a SceneComponents,
    camera_gpu: &'a CameraBindgroup,
    resources: &'a GPUResourceCache,
    pass_info: &'a PassTargetFormatInfo,
  ) {
    let material = components.materials.get(self.material()).unwrap().as_ref();
    let node = components.nodes.get_node(self.node()).data();
    let mesh = components.meshes.get(self.mesh()).unwrap().as_ref();

    let ctx = SceneMaterialPassSetupCtx {
      pass: pass_info,
      camera_gpu,
      model_gpu: node.gpu.as_ref().unwrap().into(),
      resources,
      active_mesh: mesh.into(),
    };
    material.setup_pass(pass, &ctx);

    let mesh = components.meshes.get(self.mesh()).unwrap();
    mesh.setup_pass_and_draw(pass, self.group());
  }
}

impl Scene {
  pub fn add_model(&mut self, model: impl Model + 'static) -> ModelHandle {
    self.models.insert(Box::new(model))
  }
}

pub struct MeshModel<Ma, Me> {
  pub material: TypedMaterialHandle<Ma>,
  pub mesh: TypedMeshHandle<Me>,
  pub group: MeshDrawGroup,
  pub node: SceneNodeHandle,
}

impl<Ma, Me> Model for MeshModel<Ma, Me>
where
  // constrain the model's mesh gpu layout and material requirement must be same
  Me: GPUMeshLayoutSupport,
  Ma: MaterialMeshLayoutRequire<VertexInput = Me::VertexInput>,
{
  fn material(&self) -> MaterialHandle {
    self.material.handle
  }

  fn mesh(&self) -> MeshHandle {
    self.mesh.handle
  }

  fn group(&self) -> MeshDrawGroup {
    self.group
  }

  fn node(&self) -> SceneNodeHandle {
    self.node
  }
}
