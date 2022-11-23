use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

pub enum SceneModelType {
  Standard(StandardModel),
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
