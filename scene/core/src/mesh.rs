use crate::*;

pub enum SceneMesh {
  Mesh,
  Foreign(Box<dyn ForeignImplemented>),
}
