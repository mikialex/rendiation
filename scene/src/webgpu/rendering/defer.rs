pub struct DeferPassDispatcher {
  //
}

impl PassDispatcher for DeferPassDispatcher {
  fn build_pipeline(&self, builder: &mut PipelineBuilder) {
    // builder
    //   .include_fragment_entry(
    //     "
    // [[stage(fragment)]]
    // fn fs_highlight_mask_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
    //     return vec4<f32>(1.);
    // }}
    // ",
    //   )
    //   .use_fragment_entry("fs_highlight_mask_main");
  }
}

pub struct MaterialDeferPassResult {
  world_position: Attachment,
  depth: Attachment,
  normal: Attachment,
  material: Attachment,
}

pub fn defer(engine: &RenderEngine) -> MaterialDeferPassResult {
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

