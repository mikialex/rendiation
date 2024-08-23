use crate::*;

#[derive(Default)]
pub struct SceneReaderForGltf {
  //
}

impl SceneReaderForGltf {
  pub fn read_pbr_mr_material(
    &self,
    id: EntityHandle<PbrMRMaterialEntity>,
  ) -> PhysicalMetallicRoughnessMaterialDataView {
    todo!()
  }
}
