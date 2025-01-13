use rendiation_shader_library::sampling::{hammersley_2d_fn, sample_hemisphere_cos_fn, tbn_fn};
use rendiation_texture_core::{Size, TextureSampler};
use rendiation_texture_gpu_base::SamplerConvertExt;

use crate::*;

struct BrdfLUTGenerator;
impl ShaderPassBuilder for BrdfLUTGenerator {}
impl ShaderHashProvider for BrdfLUTGenerator {
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for BrdfLUTGenerator {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let sample_count = val(128);
      let uv = builder.query::<FragmentUv>();
      let result = integrate_brdf(uv.x(), uv.y(), sample_count);
      builder.store_fragment_out(0, (result, val(0.), val(1.)))
    })
  }
}

pub fn generate_brdf_lut(encoder: &mut GPUCommandEncoder, gpu: &GPU, target: GPU2DTextureView) {
  pass("brdf lut generate")
    .with_color(target, load())
    .render(encoder, gpu)
    .by(&mut BrdfLUTGenerator.draw_quad());
}

#[derive(Clone)]
pub struct PreFilterMapGenerationResult {
  pub diffuse: GPUCubeTextureView,
  pub specular: GPUCubeTextureView,
}

pub struct PreFilterMapGenerationConfig {
  pub specular_resolution: u32,
  pub specular_sample_count: u32,
  pub diffuse_sample_count: u32,
  pub diffuse_resolution: u32,
}

pub fn generate_pre_filter_map(
  encoder: &mut GPUCommandEncoder,
  gpu: &GPU,
  input: GPUCubeTextureView,
  config: PreFilterMapGenerationConfig,
) -> PreFilterMapGenerationResult {
  let diffuse = create_cube(
    &gpu.device,
    config.diffuse_resolution,
    MipLevelCount::EmptyMipMap,
  );
  for (idx, direction) in face_direction_iter() {
    let target = cube_face_view(&diffuse, idx as u32, 0);
    let config = create_uniform(
      DiffuseTaskGenerationConfig {
        direction,
        sample_count: config.diffuse_sample_count,
        ..Default::default()
      },
      &gpu.device,
    );

    pass("prefilter diffuse env map")
      .with_color(target, load())
      .render(encoder, gpu)
      .by(
        &mut PreFilterDiffuseTask {
          input: input.clone(),
          config,
        }
        .draw_quad(),
      );
  }

  let specular = create_cube(
    &gpu.device,
    config.specular_resolution,
    MipLevelCount::BySize,
  );
  let spec_res = config.specular_resolution;
  let res = Size::from_u32_pair_min_one((spec_res, spec_res));
  let mip_level_count = MipLevelCount::BySize.get_level_count_wgpu(res);

  for (idx, direction) in face_direction_iter() {
    for level in 0..mip_level_count {
      let target = cube_face_view(&specular, idx as u32, level);
      let config = create_uniform(
        SpecularGenerationConfig {
          direction,
          roughness: (level as f32) / (mip_level_count as f32 - 1.0).clamp(0.001, 1.0),
          sample_count: config.specular_sample_count,
          ..Default::default()
        },
        &gpu.device,
      );

      pass("prefilter specular env map")
        .with_color(target, load())
        .render(encoder, gpu)
        .by(
          &mut PreFilterSpecularTask {
            input: input.clone(),
            config,
          }
          .draw_quad(),
        );
    }
  }

  PreFilterMapGenerationResult { diffuse, specular }
}

fn cube_face_view(cube: &GPUCubeTextureView, face_idx: u32, level: u32) -> GPU2DTextureView {
  let view = cube.0.resource.create_view(TextureViewDescriptor {
    label: None,
    format: None,
    dimension: Some(TextureViewDimension::D2),
    aspect: TextureAspect::All,
    base_mip_level: level,
    mip_level_count: Some(1),
    base_array_layer: face_idx,
    array_layer_count: Some(1),
  });

  GPU2DTextureView::try_from(view).unwrap()
}

fn create_cube(device: &GPUDevice, resolution: u32, level: MipLevelCount) -> GPUCubeTextureView {
  let size_ = Size::from_u32_pair_min_one((resolution, resolution));
  let size = Extent3d {
    width: resolution,
    height: resolution,
    depth_or_array_layers: 6,
  };
  let output = GPUTexture::create(
    TextureDescriptor {
      label: None,
      size,
      mip_level_count: level.get_level_count_wgpu(size_),
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8UnormSrgb,
      usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[],
    },
    device,
  );
  let output_view = output.create_view(TextureViewDescriptor {
    label: None,
    format: None,
    dimension: Some(TextureViewDimension::Cube),
    aspect: TextureAspect::All,
    base_mip_level: 0,
    mip_level_count: None,
    base_array_layer: 0,
    array_layer_count: None,
  });
  GPUCubeTextureView::try_from(output_view).unwrap()
}

fn face_direction_iter() -> impl Iterator<Item = (usize, Mat4<f32>)> {
  let eye = Vec3::new(0., 0., 0.);
  [
    Mat4::lookat(eye, Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.)), // positive x
    Mat4::lookat(eye, Vec3::new(-1., 0., 0.), Vec3::new(0., 1., 0.)), // negative x
    Mat4::lookat(eye, Vec3::new(0., 1., 0.), Vec3::new(0., 0., 1.)), // positive y
    Mat4::lookat(eye, Vec3::new(0., -1., 0.), Vec3::new(0., 0., -1.)), // negative y
    Mat4::lookat(eye, Vec3::new(0., 0., 1.), Vec3::new(0., 1., 0.)), // positive z
    Mat4::lookat(eye, Vec3::new(0., 0., -1.), Vec3::new(0., 1., 0.)), // negative z
  ]
  .into_iter()
  .enumerate()
}

struct PreFilterSpecularTask {
  input: GPUCubeTextureView,
  config: UniformBufferDataView<SpecularGenerationConfig>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
struct SpecularGenerationConfig {
  pub direction: Mat4<f32>,
  pub sample_count: u32,
  pub roughness: f32,
}

impl ShaderHashProvider for PreFilterSpecularTask {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for PreFilterSpecularTask {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.config);
    ctx.binding.bind(&self.input);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl GraphicsShaderProvider for PreFilterSpecularTask {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binder| {
      let config = binder.bind_by(&self.config).load().expand();
      let input = binder.bind_by(&self.input);
      let sampler = binder.bind_by(&ImmediateGPUSamplerViewBind);

      let resolution = builder.query::<RenderBufferSize>().x();
      let uv = builder.query::<FragmentUv>();
      let cube_face_local = uv * val(2.0) - val(Vec2::one());
      let cube_face = config.direction.shrink_to_3() * (cube_face_local, val(1.)).into();
      // is the uv interpolate from the pixel center?? i don't care
      let pixel_center_direction = cube_face.normalize();

      let specular = prefilter_specular(
        input,
        sampler,
        pixel_center_direction,
        resolution,
        config.roughness,
        config.sample_count,
      );

      builder.store_fragment_out(0, (specular, val(1.)));
    });
  }
}

pub fn prefilter_specular(
  env: HandleNode<ShaderTextureCube>,
  sampler: HandleNode<ShaderSampler>,
  normal: Node<Vec3<f32>>,
  resolution: Node<f32>,
  roughness: Node<f32>,
  sampler_count: Node<u32>,
) -> Node<Vec3<f32>> {
  let tbn = tbn_fn(normal);
  let roughness2 = roughness * roughness;

  let result = sampler_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sampler_count);
      let half = tbn * hemisphere_importance_sample_dggx(random, roughness2);
      let n_dot_h = normal.dot(half);
      let light = (val(2.) * n_dot_h * half - normal).normalize();
      let n_dot_l = normal.dot(light).max(0.);

      n_dot_l.greater_than(0.).select_branched(
        || {
          let pdf = d_ggx(n_dot_h, roughness2) / val(4.) + val(0.0001);
          // solid angle by this sample
          let omega_s = val(1.0) / (sampler_count.into_f32() * pdf);
          // solid angle covered by one pixel
          let omega_p = val(4. * f32::PI()) / (val(6.0) * resolution * resolution);
          let mip_level = (val(0.5) * (omega_s / omega_p).log2() + val(1.)).max(0.);

          let sample = env
            .build_sample_call(sampler, light)
            .with_level(mip_level)
            .sample()
            .xyz()
            * n_dot_l;
          vec4_node((sample, n_dot_l))
        },
        || val(Vec4::zero()),
      )
    })
    .sum();

  result.xyz() / result.w().splat()
}

struct PreFilterDiffuseTask {
  input: GPUCubeTextureView,
  config: UniformBufferDataView<DiffuseTaskGenerationConfig>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
struct DiffuseTaskGenerationConfig {
  pub direction: Mat4<f32>,
  pub sample_count: u32,
}

impl ShaderHashProvider for PreFilterDiffuseTask {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for PreFilterDiffuseTask {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.config);
    ctx.binding.bind(&self.input);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl GraphicsShaderProvider for PreFilterDiffuseTask {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binder| {
      let config = binder.bind_by(&self.config).load().expand();
      let input = binder.bind_by(&self.input);
      let sampler = binder.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let cube_face_local = uv * val(2.) - val(Vec2::one());
      let cube_face = config.direction.shrink_to_3() * (cube_face_local, val(1.)).into();
      // is the uv interpolate from the pixel center?? i don't care
      let pixel_center_direction = cube_face.normalize();

      let specular = prefilter_diffuse(input, sampler, pixel_center_direction, config.sample_count);

      builder.store_fragment_out(0, (specular, val(1.)));
    });
  }
}

pub fn prefilter_diffuse(
  env: HandleNode<ShaderTextureCube>,
  sampler: HandleNode<ShaderSampler>,
  normal: Node<Vec3<f32>>,
  sampler_count: Node<u32>,
) -> Node<Vec3<f32>> {
  let tbn = tbn_fn(normal);
  sampler_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sampler_count);
      let light = tbn * sample_hemisphere_cos_fn(random);
      let n_dot_l = normal.dot(light).max(0.);
      n_dot_l.greater_than(0.).select(
        env.sample_zero_level(sampler, light).xyz(),
        val(Vec3::zero()),
      )
    })
    .sum()
    / sampler_count.into_f32().splat()
}
