//! Draw tests: the fragment inputs of the trimming pipeline variant (plan-trim T2 plumbing).

use parametric_normalized_model::*;
use parametric_tessellation::test_scenes::*;
use parametric_tessellation::*;

use super::*;
use crate::*;

/// The parameter and the item received by the fragments of the trimmed buckets: the surface
/// point of the parameter must project to the pixel center up to the tessellation error.
#[pollster::test]
async fn trimmed_fragment_receives_exact_parameter() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let model = half_trimmed(random_model(11, 12, (3., 15.)));
  let camera = Camera {
    view: Mat4::identity(),
    focal_length: 200.,
    viewport: Vec2::new(400., 300.),
    near: 0.1,
  };
  let config = TessellationConfig::default();
  let mut t = GpuTessellator::new(&gpu, &model, config);
  let mut encoder = gpu.create_encoder();
  t.encode_frame(&gpu, &mut encoder, &camera);
  gpu.submit_encoder(encoder);
  let frame = t.read_back(&gpu).await;
  assert_eq!(frame.stats.overflow, 0);
  assert!(frame.stats.trimmed_leaves > 0 && frame.stats.trimmed_leaves < frame.stats.leaves);

  let pixels = render_parameters(&gpu, &camera, t.output()).await;

  let items = camera_space_items(&model, &camera, |_| true).0;
  let (width, height) = (camera.viewport.x as usize, camera.viewport.y as usize);
  let (mut trimmed, mut untrimmed, mut max_offset) = (0, 0, 0f64);
  for (index, p) in pixels.iter().enumerate() {
    match p.w {
      1. => untrimmed += 1,
      2. => {
        trimmed += 1;
        let item = &items[p.z as usize];
        assert!(!model.patches[item.patch as usize].is_untrimmed());
        let s = item.surface.eval(p.x as f64, p.y as f64);
        let pixel = Vec2::new((index % width) as f64, (index / width) as f64) + Vec2::splat(0.5);
        let projected = Vec2::new(s.x, -s.y) * (camera.focal_length / -s.z) + camera.viewport * 0.5;
        max_offset = max_offset.max((projected - pixel).length());
      }
      _ => {}
    }
  }
  println!("trimmed {trimmed} untrimmed {untrimmed} pixels, max offset {max_offset} px");
  assert!(trimmed > 1000 && untrimmed > 1000);
  assert!(max_offset < config.pixel_error + 0.05, "{max_offset}");
  assert_eq!(pixels.len(), width * height);
}

/// Every second patch gets a trim loop with an edge curve (the loop stays the patch boundary).
fn half_trimmed(mut model: ParametricNormalizedModel) -> ParametricNormalizedModel {
  for patch in model.patches.iter_mut().skip(1).step_by(2) {
    patch.loops[0].curves[0].source = TrimCurveSource::Edge {
      edge: EdgeId(0),
      reversed: false,
    };
  }
  model
}

/// Draws `output` into an `Rgba32Float` target, the trimmed buckets write `(u, v, item, 2)`,
/// the others their shaded color with alpha 1.
async fn render_parameters(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
) -> Vec<Vec4<f32>> {
  let size = Size::from_u32_pair_min_one((camera.viewport.x as u32, camera.viewport.y as u32));
  let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
  let target = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Rgba32Float);

  // camera space to clip space, depth `near / -z`
  let (sx, sy) = (
    (camera.focal_length / (camera.viewport.x * 0.5)) as f32,
    (camera.focal_length / (camera.viewport.y * 0.5)) as f32,
  );
  #[rustfmt::skip]
  let clip = Mat4::new(
    sx, 0., 0., 0.,
    0., sy, 0., 0.,
    0., 0., 0., -1.,
    0., 0., camera.near as f32, 0.,
  );
  let settings = TessellationDrawSettings {
    trimmed_style: TrimmedPatchStyle::Trimmed,
    ..Default::default()
  };
  let uniform = TessellationDrawUniform::new(clip, Mat4::identity(), &settings, output);
  let uniform = create_uniform(uniform, &gpu.device, "test draw uniform");

  let mut encoder = gpu.create_encoder();
  let desc = pass("parameter test").with_color(&target.clone().into(), clear_and_store(all_zero()));
  let pass = encoder.begin_render_pass(&desc, None);
  let mut ctx = GPURenderPassCtx::new(pass, gpu.clone());
  let trim = ParameterOutput;
  render_tessellation_output(
    &mut ctx,
    output,
    &uniform,
    Some(&trim),
    &[&FloatTarget],
    &[],
  );
  drop(ctx);
  let read = encoder.read_texture_2d(
    &gpu.device,
    &target,
    ReadRange {
      size,
      offset_x: 0,
      offset_y: 0,
    },
  );
  gpu.submit_encoder(encoder);
  let read = read.await.unwrap();
  cast_slice(&read.read_into_raw_unpadded_buffer()).to_vec()
}

/// Declares the single `Rgba32Float` target and writes `DefaultDisplay` to it.
struct FloatTarget;
impl ShaderHashProvider for FloatTarget {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for FloatTarget {}
impl GraphicsShaderProvider for FloatTarget {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(TextureFormat::Rgba32Float));
    });
  }
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let display = builder.query::<DefaultDisplay>();
      builder.store_fragment_out(0, display);
    });
  }
}

/// A trim fragment that only overrides the display with its inputs.
struct ParameterOutput;
impl ShaderHashProvider for ParameterOutput {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for ParameterOutput {}
impl TessellationTrimFragment for ParameterOutput {
  fn build_trim(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    _: &mut ShaderBindGroupBuilder,
    uv: Node<Vec2<f32>>,
    item: Node<u32>,
  ) {
    let display: Node<Vec4<f32>> = (uv, item.into_f32(), val(2.)).into();
    builder.register::<DefaultDisplay>(display);
  }
}
