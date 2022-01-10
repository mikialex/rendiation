use crate::MeshModelImpl;

pub struct ModelCollection<Ma, Me> {
  pub models: Vec<MeshModelImpl<Ma, Me>>,
}
