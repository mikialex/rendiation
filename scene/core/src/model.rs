use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[non_exhaustive]
pub enum SceneModelType {
  Standard(SceneItemRef<StandardModel>),
  Foreign(Box<dyn ForeignImplemented>),
}

pub type SceneModel = SceneItemRef<SceneModelImpl>;

pub struct SceneModelImpl {
  pub model: SceneModelType,
  pub node: SceneNode,
}

pub struct StandardModel {
  pub material: SceneMaterial,
  pub mesh: SceneMesh,
  pub group: MeshDrawGroup,
}
