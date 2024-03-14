pub struct BoundingBox {
  model: HelperLineModel,
  mode: BoundingMode,
}

pub enum BoundingMode {
  LocalSpace,
  WorldSpace,
}
