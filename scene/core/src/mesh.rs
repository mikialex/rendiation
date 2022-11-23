use crate::*;

pub type SceneMesh = SceneItemRef<SceneMeshType>;

pub enum SceneMeshType {
  Mesh,
  Foreign(Box<dyn ForeignImplemented>),
}
