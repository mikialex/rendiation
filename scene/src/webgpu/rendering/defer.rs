pub struct MaterialDeferPass {
  //
}

pub struct MaterialDeferPassResult {
  world_position: Attachment,
  depth: Attachment,
  normal: Attachment,
  material: Attachment,
}

pub fn defer() -> MaterialDeferPassResult {
  todo!()
}
