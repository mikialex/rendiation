pub struct DeferPassDispatcher {
  //
}

pub struct MaterialDeferPassResult {
  world_position: Attachment,
  depth: Attachment,
  normal: Attachment,
  material: Attachment,
}

pub fn defer(ctx: &RenderEngine) -> MaterialDeferPassResult {
  todo!()
}

pub trait DeferShading: LightableSurfaceShading {
  fn decode_geometry_ctx_from_g_buffer(&self) -> ShaderString;
  fn decode_material_from_g_buffer(&self) -> ShaderString;
  fn encode_g_buffer(
    &self,
    builder: &mut ShaderBuilder,
    ctx: &ShaderConstructCtx,
    source: &dyn ShaderComponent,
  );
}
