use super::*;

// Inner has natural WGSL alignment 4, but std140 requires the nested struct member aligned to
// 16, so the Outer layout is not natural and explicit padding members are required.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct LayoutTestInner {
  pub x: f32,
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy)]
pub struct LayoutTestOuter {
  pub a: f32,
  pub inner: LayoutTestInner,
  pub b: Vec2<f32>,
  pub arr: Shader140Array<LayoutTestInner, 2>,
  pub c: f32,
}

fn test_data() -> LayoutTestOuter {
  let inner = |x| {
    let mut v: LayoutTestInner = Zeroable::zeroed();
    v.x = x;
    v
  };
  let mut data: LayoutTestOuter = Zeroable::zeroed();
  data.a = 1.;
  data.inner = inner(2.);
  data.b = Vec2::new(3., 4.);
  data.arr = [inner(5.), inner(6.)].into();
  data.c = 7.;
  data
}

const EXPECTED: [f32; 11] = [1., 2., 3., 4., 5., 6., 7., 2., 107., 2., 4.];

fn build_test_shader(
  gpu: &GPU,
  input: &UniformBufferDataView<LayoutTestOuter>,
  output: &StorageBufferDataView<[f32]>,
) -> ShaderComputePipelineBuilder {
  let mut cx = compute_shader_builder(gpu).with_config_work_group_size(1);
  let input = cx.bind_by(input);
  let output = cx.bind_by(output);
  let write = |i: u32, v: Node<f32>| output.index(val(i)).store(v);

  // field access through the uniform pointer
  write(0, input.a().load());
  write(1, input.inner().x().load());
  write(2, input.b().load().x());
  write(3, input.b().load().y());
  write(4, input.arr().index(val(0)).x().load());
  write(5, input.arr().index(val(1)).x().load());
  write(6, input.c().load());

  // field access on the loaded value
  let loaded = input.load().expand();
  write(7, LayoutTestInner::x(loaded.inner));

  // compose a padded struct, then access it through a local pointer and as value
  let rebuilt = ENode::<LayoutTestOuter> {
    c: loaded.c + val(100.),
    ..loaded
  }
  .construct();
  let local = rebuilt.make_local_var();
  write(8, local.c().load());
  write(9, local.inner().x().load());
  write(10, rebuilt.expand().b.y());

  cx
}

/// When `via_wgsl_text` is true, the naga module is converted to WGSL text first, then the text
/// is used to create the shader module. This is exactly what the browser WebGPU backend does,
/// in this case the layout is recomputed from the member types and the offsets in naga IR are
/// dropped.
async fn run_layout_test(via_wgsl_text: bool) {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let input = create_uniform(test_data(), &gpu, "layout test input");
  let output = create_gpu_read_write_storage::<[f32]>(
    ZeroedArrayByArrayLength(EXPECTED.len()),
    &gpu,
    "layout test output",
  );

  let result = build_test_shader(&gpu, &input, &output).build().unwrap();
  let (entry, shader) = result.shader;
  let naga_module = shader.downcast::<NagaModuleBuildResult>().unwrap().module;

  let module = if via_wgsl_text {
    let text = convert_module_by_wgsl(&naga_module, naga::valid::ValidationFlags::all());
    assert!(
      text.contains("padding_"),
      "explicit padding members are expected:\n{text}"
    );
    gpu
      .device
      .create_shader_module(gpu::ShaderModuleDescriptor {
        label: None,
        source: gpu::ShaderSource::Wgsl(text.into()),
      })
  } else {
    gpu.device.create_shader_module_by_shader_api(
      NagaModuleBuildResult {
        log_result: false,
        module: naga_module,
      },
      ShaderRuntimeChecks::default(),
    )
  };

  let (raw_layouts, layouts, pipeline_layout) = create_layouts(&gpu.device, &result.bindings);
  let pipeline = gpu
    .device
    .create_compute_pipeline(&gpu::ComputePipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      module: &module,
      entry_point: Some(&entry),
      compilation_options: Default::default(),
      cache: None,
    });
  let pipeline = GPUPipeline::new(pipeline, raw_layouts, layouts);

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(&input)
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);
  });

  let result = encoder.read_buffer(&gpu.device, &output);
  gpu.submit_encoder(encoder);
  let result = result.await.unwrap();
  let result = <[f32]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
  assert_eq!(result, EXPECTED);
}

#[pollster::test]
async fn test_std140_nested_struct_layout_by_naga_ir() {
  run_layout_test(false).await;
}

#[pollster::test]
async fn test_std140_nested_struct_layout_by_wgsl_text() {
  run_layout_test(true).await;
}
