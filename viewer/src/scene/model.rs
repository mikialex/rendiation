use std::marker::PhantomData;

use arena::Arena;
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

pub struct ModelPassSetupContext<'a> {
  pub materials: &'a Arena<Box<dyn Material>>,
  pub meshes: &'a Arena<Box<dyn Mesh>>,
  pub material_ctx: SceneMaterialPassSetupCtx<'a>,
}

impl Scene {
  pub fn add_model(&mut self, model: impl Model + 'static) -> ModelHandle {
    self.models.insert(Box::new(model))
  }
}
