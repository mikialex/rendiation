use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[non_exhaustive]
#[derive(Clone)]
pub enum SceneModelType {
  Standard(SceneItemRef<StandardModel>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

clone_self_incremental!(SceneModelType);

pub type SceneModel = SceneItemRef<SceneModelImpl>;

#[derive(Incremental)]
pub struct SceneModelImpl {
  pub model: SceneModelType,
  pub node: SceneNode,
}

#[derive(Incremental)]
pub struct StandardModel {
  pub material: SceneMaterialType,
  pub mesh: SceneMeshType,
  pub group: MeshDrawGroup,
}
