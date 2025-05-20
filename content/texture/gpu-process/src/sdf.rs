use crate::*;

/// reference: https://github.com/rerun-io/rerun/blob/main/crates/viewer/re_renderer/src/draw_phases/outlines.rs
pub fn compute_sdf(
  frame_cx: &mut FrameCtx,
  mask_input: GPU2DTextureView,
  max_distance: Option<u32>,
) -> GPU2DTextureView {
  let size = mask_input.size();
  let (width, height) = size.into_u32();
  let max_distance = max_distance.unwrap_or(width.max(height));
  let max_step_width = max_distance.max(1).next_power_of_two();
  let num_steps = max_step_width.ilog2() + 1;

  let fmt = TextureFormat::Rg32Float;
  let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
  let mut source = create_empty_2d_texture_view(frame_cx.gpu, size, usage, fmt);
  let mut target = create_empty_2d_texture_view(frame_cx.gpu, size, usage, fmt);

  pass("jump flooding sdf compute init")
    .with_color(&RenderTargetView::Texture(source.clone().texture), load())
    .render_ctx(frame_cx)
    .by(
      &mut JumpFloodingInit {
        mask_source: mask_input.clone(),
      }
      .draw_quad(),
    );

  for i in 0..num_steps {
    let step = create_uniform(
      Vec4::<u32>::new(max_step_width >> i, 0, 0, 0),
      &frame_cx.gpu.device,
    );

    pass("jump flooding sdf compute iteration")
      .with_color(&RenderTargetView::Texture(target.clone().texture), load())
      .render_ctx(frame_cx)
      .by(
        &mut JumpFlooding {
          source: source.clone(),
          step,
        }
        .draw_quad(),
      );

    // ping pong;
    std::mem::swap(&mut source, &mut target);
  }

  target
}

struct JumpFloodingInit {
  mask_source: GPU2DTextureView,
}

impl ShaderHashProvider for JumpFloodingInit {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for JumpFloodingInit {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let mask = binding.bind_by(&self.mask_source);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let size = builder.query::<RenderBufferSize>();
      let pixel_coordinates = (size * uv).floor();

      let is_mask = mask.sample_zero_level(sampler, uv).x().not_equals(0.);

      let init = is_mask.select(pixel_coordinates, val(Vec2::splat(f32::MAX)));

      builder.store_fragment_out_vec4f(0, (init, val(0.), val(1.)));
    })
  }
}

impl ShaderPassBuilder for JumpFloodingInit {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.mask_source);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

struct JumpFlooding {
  source: GPU2DTextureView,
  step: UniformBufferDataView<Vec4<u32>>,
}

impl ShaderHashProvider for JumpFlooding {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for JumpFlooding {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let source = binding.bind_by(&self.source);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let step = binding.bind_by(&self.step).load().x();
      let step = vec2_node((step, step));

      let resolution = source.texture_dimension_2d(None);
      let pixel_step = step.into_f32() / resolution.into_f32();

      let uv = builder.query::<FragmentUv>();
      let size = builder.query::<RenderBufferSize>();
      let pixel_coordinates = (size * uv).floor();

      let closest_positions = val(Vec2::splat(f32::MAX)).make_local_var();
      let closest_distance_sq_a = val(f32::MAX).make_local_var();

      for i in -1..1 {
        for j in -1..1 {
          let texcoord = uv + val(Vec2::new(i as f32, j as f32)) * pixel_step;
          let position = source.sample_zero_level(sampler, texcoord).xy();
          if_by(position.equals(Vec2::splat(f32::MAX)).all().not(), || {
            let position_delta = position - pixel_coordinates;

            let distance_sq = position_delta.dot(position_delta);
            if_by(
              closest_distance_sq_a.load().greater_than(distance_sq),
              || {
                closest_distance_sq_a.store(distance_sq);
                closest_positions.store(position);
              },
            );
          });
        }
      }

      builder.store_fragment_out_vec4f(0, (closest_positions.load(), val(0.), val(1.)));
    })
  }
}

impl ShaderPassBuilder for JumpFlooding {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.source);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(&self.step);
  }
}
