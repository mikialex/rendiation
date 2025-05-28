use rendiation_shader_library::{
  normal_mapping::compute_normal_by_dxdy, sampling::*, shader_uv_space_to_world_space,
  shader_world_space_to_uv_space,
};

use crate::*;

// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/sections/ssao.md

const MAX_SAMPLE: usize = 64;

pub struct SSAO {
  parameters: UniformBufferCachedDataView<SSAOParameter>,
  samples: UniformBufferCachedDataView<Shader140Array<Vec4<f32>, MAX_SAMPLE>>,
}

fn rand() -> f32 {
  rand::random()
}

impl SSAO {
  pub fn new(gpu: &GPU) -> Self {
    let parameters = SSAOParameter::default();

    // improve, try other low discrepancy serials
    let samples: Vec<Vec4<f32>> = (0..MAX_SAMPLE)
      .map(|i| {
        // generate point in half sphere
        let rand_p = loop {
          let rand_p = Vec3::new(rand() * 2. - 1., rand() * 2. - 1., rand());
          if rand_p.length() < 1. {
            break rand_p;
          }
        };
        let rand_p = rand_p.expand_with_one();
        let scale = (i as f32) / (parameters.sample_count as f32);
        rand_p * scale
      })
      .collect();
    let samples = samples.try_into().unwrap();
    let samples = create_uniform_with_cache(samples, gpu);

    let parameters = create_uniform_with_cache(parameters, gpu);

    Self {
      parameters,
      samples,
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct SSAOParameter {
  pub noise_size: u32,
  pub sample_count: u32,
  pub radius: f32,
  pub bias: f32,
  pub magnitude: f32,
  pub contrast: f32,
  pub noise_jit: f32,
}

impl Default for SSAOParameter {
  fn default() -> Self {
    Self {
      noise_size: 64,
      sample_count: 32,
      radius: 2.,
      bias: 0.0001,
      magnitude: 1.0,
      contrast: 1.5,
      noise_jit: 0.,
      ..Zeroable::zeroed()
    }
  }
}

pub struct AOComputer<'a> {
  reverse_depth: bool,
  depth: &'a RenderTargetView,
  parameter: &'a SSAO,
  reproject: &'a UniformBufferCachedDataView<ReprojectInfo>,
}

impl ShaderHashProvider for AOComputer<'_> {
  shader_hash_type_id! {AOComputer<'static>}
}

impl ShaderPassBuilder for AOComputer<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.depth);
    ctx.binding.bind(&self.parameter.parameters);
    ctx.binding.bind(&self.parameter.samples);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(self.reproject);
  }
}
impl GraphicsShaderProvider for AOComputer<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let depth_tex = binding.bind_by(&DisableFiltering(&self.depth));
      let parameter = binding.bind_by(&self.parameter.parameters).load().expand();
      let samples = binding.bind_by(&self.parameter.samples);
      let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));

      let reproject = binding.bind_by(self.reproject).load().expand();

      let uv = builder.query::<FragmentUv>();

      let sample_count_f = parameter.sample_count.into_f32();

      let depth = depth_tex.sample(sampler, uv).x();

      let is_background = if self.reverse_depth {
        depth.equals(val(0.))
      } else {
        depth.equals(val(1.))
      };

      if_by(is_background, || {
        builder.store_fragment_out_vec4f(0, Vec4::one());
      })
      .else_by(|| {
        let position_world =
          shader_uv_space_to_world_space(reproject.current_camera_view_projection_inv, uv, depth);

        let normal = compute_normal_by_dxdy(position_world); // wrong, but i do not want pay cost to use normal texture input

        let random = random3_fn(uv + parameter.noise_jit.splat()) * val(2.) - val(Vec3::one());
        let tangent = (random - normal * random.dot(normal)).normalize();
        let binormal = normal.cross(tangent);
        let tbn = mat3_node((tangent, binormal, normal));

        let occlusion_sum = samples
          .into_shader_iter()
          .clamp_by(parameter.sample_count)
          .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
            let sample_position_offset = tbn * sample.load().xyz();
            let sample_position_world = position_world + sample_position_offset * parameter.radius;

            let (s_uv, s_depth) = shader_world_space_to_uv_space(
              reproject.current_camera_view_projection,
              sample_position_world,
            );
            let sample_position_depth = depth_tex.sample_zero_level(sampler, s_uv).x();

            let occluded = if self.reverse_depth {
              (sample_position_depth + parameter.bias)
                .greater_than(s_depth)
                .select(0., 1.)
            } else {
              (sample_position_depth + parameter.bias)
                .less_than(s_depth)
                .select(0., 1.)
            };

            let relative_depth_diff = parameter.radius / (sample_position_depth - s_depth).abs();
            let intensity = relative_depth_diff.smoothstep(val(0.), val(1.));

            occluded * intensity
          })
          .sum();

        let occlusion = parameter.sample_count.into_f32() - occlusion_sum;
        let occlusion = occlusion / sample_count_f;
        let occlusion = occlusion.pow(parameter.magnitude);
        let occlusion = parameter.contrast * (occlusion - val(0.5)) + val(0.5);

        builder.store_fragment_out_vec4f(0, ((val(1.) - occlusion.saturate()).splat(), val(1.)))
      });
    })
  }
}

impl SSAO {
  pub fn draw(
    &self,
    ctx: &mut FrameCtx,
    depth: &RenderTargetView,
    reproject: &UniformBufferCachedDataView<ReprojectInfo>,
    reverse_depth: bool,
  ) -> RenderTargetView {
    self.parameters.mutate(|p| p.noise_jit = rand());
    self.parameters.upload(&ctx.gpu.queue);

    let ao_result = attachment()
      .sizer(ratio_sizer(0.5)) // half resolution!
      .format(TextureFormat::Rgba8Unorm) // todo single channel
      .request(ctx);

    pass("ssao-compute")
      .with_color(&ao_result, store_full_frame())
      .render_ctx(ctx)
      .by(
        &mut AOComputer {
          reproject,
          depth,
          parameter: self,
          reverse_depth,
        }
        .draw_quad(),
      );

    // todo blur

    ao_result
  }
}
