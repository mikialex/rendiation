use rendiation_shader_library::{shader_render_space_to_uv_space, shader_uv_space_to_render_space};

use crate::*;

// https://lettier.github.io/3d-game-shaders-for-beginners/ssao.html

const MAX_SAMPLE: usize = 64;
const NOISE_TEX_SIZE: u32 = 16;

pub struct SSAO {
  parameters: UniformBufferCachedDataView<SSAOParameter>,
  samples: UniformBufferCachedDataView<Shader140Array<Vec4<f32>, MAX_SAMPLE>>,
  noise_texture: GPU2DTextureView,
  blur_data: BilateralBlurData,
}

fn radical_inverse_vdc(bits: u32) -> f32 {
  let bits = (bits << 16) | (bits >> 16);
  let bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
  let bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
  let bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
  let bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
  bits as f32 * 2.328_306_4e-10
}

fn prefix_stable_2d(i: u32) -> (f32, f32) {
  let u = radical_inverse_vdc(i + 1);
  let v = ((i as f32 + 0.5) * 0.618_034).fract();
  (u, v)
}

fn sample_hemisphere_uniform(uv: (f32, f32)) -> (f32, f32, f32) {
  let phi = 2.0 * std::f32::consts::PI * uv.1;
  let cos_theta = uv.0;
  let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
  (phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta)
}

fn sample_radius(i: u32) -> f32 {
  let u = ((i as f32 + 0.5) * 0.754_877_7).fract();
  0.1 + 0.9 * u * u
}

fn rand_f() -> f32 {
  rand::random()
}

impl SSAO {
  pub fn new(gpu: &GPU) -> Self {
    let parameters = SSAOParameter::default();

    // Use prefix-stable samples because the shader can consume any prefix of this array.
    let samples: Vec<Vec4<f32>> = (0..MAX_SAMPLE)
      .map(|i| {
        let i = i as u32;
        let (u, v) = prefix_stable_2d(i);
        let (x, y, z) = sample_hemisphere_uniform((u, v));
        let scale = sample_radius(i);
        Vec4::new(x * scale, y * scale, z * scale, 1.0)
      })
      .collect();
    let samples = samples.try_into().unwrap();
    let samples = create_uniform_with_cache(samples, gpu);

    let parameters = create_uniform_with_cache(parameters, gpu);

    // 4x4 noise texture for per-pixel rotation
    let noise_pixels: Vec<u8> = (0..(NOISE_TEX_SIZE * NOISE_TEX_SIZE) as usize)
      .flat_map(|_| {
        let r = (rand_f() * 255.0) as u8;
        let g = (rand_f() * 255.0) as u8;
        let b = (rand_f() * 255.0) as u8;
        [r, g, b, 255u8]
      })
      .collect();

    let noise_image = GPUBufferImage {
      data: noise_pixels,
      format: TextureFormat::Rgba8Unorm,
      size: Size::from_u32_pair_min_one((NOISE_TEX_SIZE, NOISE_TEX_SIZE)),
    };
    let noise_texture = create_gpu_texture2d(gpu, &noise_image);

    let blur_data = BilateralBlurData::new(gpu);

    Self {
      parameters,
      samples,
      noise_texture,
      blur_data,
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct SSAOParameter {
  pub sample_count: u32,
  pub radius: f32,
  pub bias: f32,
  pub magnitude: f32,
  pub contrast: f32,
  pub max_distance: f32,
  pub noise_jit: f32,
}

impl Default for SSAOParameter {
  fn default() -> Self {
    Self {
      sample_count: 64,
      radius: 2.,
      bias: 0.0001,
      magnitude: 0.8,
      contrast: 1.2,
      max_distance: 50.,
      noise_jit: 0.,
      ..Zeroable::zeroed()
    }
  }
}

pub struct AOComputer<'a> {
  reverse_depth: bool,
  depth: &'a RenderTargetView,
  normal: &'a RenderTargetView,
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
    ctx.binding.bind(&self.parameter.noise_texture);
    ctx.binding.bind(self.normal);
    let ssao_sampler = TextureSampler::default().into_gpu();
    ctx.bind_immediate_sampler(&ssao_sampler); // depth_sampler (NonFiltering)
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu()); // normal_sampler (Filtering)
    ctx.binding.bind(self.reproject);
  }
}

impl GraphicsShaderProvider for AOComputer<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let depth_tex = binding.bind_by(&DisableFiltering(&self.depth));
      let parameter = binding.bind_by(&self.parameter.parameters).load().expand();
      let samples = binding.bind_by(&self.parameter.samples);
      let noise_tex = binding.bind_by(&self.parameter.noise_texture);
      let normal_tex = binding.bind_by(self.normal);
      let depth_sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
      let normal_sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let reproject = binding.bind_by(self.reproject).load().expand();

      let uv = builder.query::<FragmentUv>();
      let texel_size = builder.query::<TexelSize>();

      let sample_count = parameter
        .sample_count
        .min(val(MAX_SAMPLE as u32))
        .max(val(1_u32));
      let sample_count_f = sample_count.into_f32();

      let depth = depth_tex.sample(depth_sampler, uv).x();

      let is_background = if self.reverse_depth {
        depth.equals(val(0.))
      } else {
        depth.equals(val(1.))
      };

      let render_position =
        shader_uv_space_to_render_space(reproject.current_camera_view_projection_inv, uv, depth);

      let normal = normal_tex.sample(normal_sampler, uv).xyz().normalize();

      if_by(is_background, || {
        builder.store_fragment_out_vec4f(0, Vec4::one());
      })
      .else_by(|| {
        let pixel_coord = uv / texel_size;
        let tile_size = val(Vec2::new(NOISE_TEX_SIZE as f32, NOISE_TEX_SIZE as f32));
        let jit = vec2_node((parameter.noise_jit, parameter.noise_jit * val(0.618)));
        let noise_tile = ((pixel_coord + jit) / tile_size).fract() * tile_size;
        let noise_idx: Node<Vec2<u32>> =
          (noise_tile.x().into_u32(), noise_tile.y().into_u32()).into();
        let random = noise_tex.load_texel(noise_idx, val(0u32)).xyz() * val(2.) - val(Vec3::one());

        let tangent = (random - normal * random.dot(normal)).normalize();
        let binormal = normal.cross(tangent);
        let tbn = mat3_node((tangent, binormal, normal));

        let radius = parameter.radius.max(val(0.0001));

        let occlusion_sum = samples
          .into_shader_iter()
          .clamp_by(sample_count)
          .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
            let sample_position_offset = tbn * sample.load().xyz();
            let sample_position_in_render = render_position + sample_position_offset * radius;

            let (s_uv, s_depth) = shader_render_space_to_uv_space(
              reproject.current_camera_view_projection,
              sample_position_in_render,
            );
            let in_screen = s_uv
              .x()
              .greater_equal_than(0.0)
              .and(s_uv.x().less_equal_than(1.0))
              .and(s_uv.y().greater_equal_than(0.0))
              .and(s_uv.y().less_equal_than(1.0));
            let in_depth_range = s_depth
              .greater_equal_than(0.0)
              .and(s_depth.less_equal_than(1.0));
            let valid_sample = in_screen.and(in_depth_range);
            let sample_position_depth = depth_tex.sample_zero_level(depth_sampler, s_uv).x();

            // I'm not sure if it's worth, should we add a switch for this?
            let sample_surface_position = shader_uv_space_to_render_space(
              reproject.current_camera_view_projection_inv,
              s_uv,
              sample_position_depth,
            );
            let sample_distance = (sample_surface_position - render_position).length();
            let range_weight = (val(1.0) - sample_distance / radius)
              .saturate()
              .smoothstep(0.0, 1.0);

            let occluded = if self.reverse_depth {
              (sample_position_depth + parameter.bias)
                .greater_than(s_depth)
                .select(1., 0.)
            } else {
              (sample_position_depth + parameter.bias)
                .less_than(s_depth)
                .select(1., 0.)
            };

            valid_sample.select(occluded * range_weight, val(0.0))
          })
          .sum();

        let occlusion = occlusion_sum / sample_count_f;
        let occlusion = occlusion.pow(parameter.magnitude);
        let occlusion = parameter.contrast * (occlusion - val(0.5)) + val(0.5);

        // Fade out SSAO at distance to avoid depth-precision artifacts
        let ao = val(1.) - occlusion.saturate();
        let fade = (render_position.length() / parameter.max_distance).saturate();
        let ao = ao + (val(1.) - ao) * fade;

        builder.store_fragment_out_vec4f(0, (ao.splat(), val(1.)))
      });
    })
  }
}

impl SSAO {
  pub fn parameters(&self) -> &UniformBufferCachedDataView<SSAOParameter> {
    &self.parameters
  }

  pub fn draw(
    &self,
    ctx: &mut FrameCtx,
    depth: &RenderTargetView,
    normal: &RenderTargetView,
    reproject: &UniformBufferCachedDataView<ReprojectInfo>,
    reverse_depth: bool,
    apply_blur: bool,
  ) -> RenderTargetView {
    self.parameters.mutate(|p| p.noise_jit = rand_f());
    self.parameters.upload(&ctx.gpu.queue);

    let ao_result = attachment()
      .sizer(ratio_sizer(0.5))
      .format(TextureFormat::Rgba8Unorm)
      .request(ctx);

    pass("ssao-compute")
      .with_color(&ao_result, store_full_frame())
      .render_ctx(ctx)
      .by(
        &mut AOComputer {
          reproject,
          depth,
          normal,
          parameter: self,
          reverse_depth,
        }
        .draw_quad(),
      );

    if apply_blur {
      draw_cross_bilateral_blur(&self.blur_data, ao_result, depth, ctx)
    } else {
      ao_result
    }
  }
}
