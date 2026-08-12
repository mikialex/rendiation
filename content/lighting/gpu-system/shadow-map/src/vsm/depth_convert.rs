use crate::*;

/// the pass that converts the rendered ndc depth of one shadow map region
/// into the linear depth moments, reads the depth atlas layer and writes
/// the moments into the vsm atlas layer
pub(super) struct VsmConvertTask<'a> {
  pub input: &'a GPU2DArrayDepthTextureView,
  pub config: &'a UniformBufferDataView<VsmMapProcessor>,
  pub map_info: &'a UniformBufferDataView<ShadowMapAddressInfo>,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for VsmConvertTask<'_> {
  shader_hash_type_id! {VsmConvertTask<'static>}
}

impl GraphicsShaderProvider for VsmConvertTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let input = binding.bind_by(self.input);
      let config = binding.bind_by(self.config).load().expand();
      let info = binding.bind_by(self.map_info).load().expand();

      let coord = builder.query::<FragmentPosition>().xy().floor().into_u32();
      let layer = info.layer_index.into_u32();
      let depth = input.load_texel_layer(coord, layer, val(0));

      let linear_depth = recover_linear_depth_fn(
        depth,
        config.proj_linear_depth_recover_helper,
        val(self.reversed_depth),
      );
      let moments = (linear_depth, linear_depth * linear_depth, val(0.), val(0.)).into();
      builder.store_fragment_out(0, moments);
    });
  }
}

impl ShaderPassBuilder for VsmConvertTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.input);
    ctx.binding.bind(self.config);
    ctx.binding.bind(self.map_info);
  }
}

/// recover the linear depth in [0, 1], 0 at the near plane and 1 at the far
/// plane, from the render space ndc depth, the render depth is first mapped
/// into the opengl ndc space with the reversed_depth flag, the helper is
/// extracted from the opengl ndc projection of the shadow camera
#[shader_fn]
pub fn recover_linear_depth(
  // the depth in the webgpu ndc space of the shadow camera, in [0, 1]
  z_render: Node<f32>,
  helper: Node<ProjLinearDepthRecoverHelper>,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let helper = helper.expand();
  let z_opengl = reversed_depth.select(val(1.) - z_render * val(2.), z_render * val(2.) - val(1.));

  let near = helper.near;
  let far = helper.far;
  let m33 = helper.w_row.y();

  // the orthographic projection has m33 = 1 and the perspective has m33 = 0,
  // for the orthographic projection the ndc depth is already linear
  let linear_ortho = (z_opengl + val(1.)) * val(0.5);

  // for the perspective projection, reconstruct the view depth from the
  // z row coefficients of the opengl ndc projection
  let range = far - near;
  let m22 = -(far + near) / range;
  let m23 = -(val(2.) * near * far) / range;
  let z_view = m23 / (-z_opengl - m22);
  let linear_persp = (-z_view - near) / range;

  m33.less_than(val(0.5)).select(linear_persp, linear_ortho)
}
