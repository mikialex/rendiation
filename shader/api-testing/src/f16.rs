//! The f16 support, the shaders require `Features::SHADER_F16`, the GPU tests are skipped when the
//! adapter does not support it.

use rendiation_shader_api::*;
use wgpu_types::Features;

use crate::binding::*;
use crate::graphics::*;
use crate::harness::*;

fn h(v: f32) -> f16 {
  f16::from_f32(v)
}

/// the f16 values that are only known at runtime
fn runtime_f16(builder: &ShaderComputePipelineBuilder) -> Node<f16> {
  runtime_values(builder).f.into_f16()
}

/// the arithmetic, comparison and selection of the f16 scalars and vectors, and the scalar vector
/// mixed operands
#[test]
fn f16_operators() {
  check_compute(|builder| {
    let s = runtime_f16(builder);
    let v2 = s.splat::<Vec2<f16>>();
    let v3: Node<Vec3<f16>> = (s, v2).into();
    let v4: Node<Vec4<f16>> = (v2, v2).into();

    keep(s + s - s * s / s % s);
    keep(-v3 + v3 * s - s / v3);
    keep(v4 % v4 + v4 * v4);
    keep(s.less_than(val_f16(1.)).and(v3.equals(v3).all()));
    keep(v4.greater_equal_than(v4).any());
    keep(s.greater_than(val(h(2.))).select(v3, -v3));
    keep(v3.less_than(v3).select_per_channel::<f16>(v3, v3));
    keep(v4.zyx() + v4.www());

    let acc = s.make_local_var();
    acc.store(acc.load() + s);
    let mut v = v3;
    v += s;
    v *= s;
    v -= v3;
    v /= s;
    keep(v);
    keep(acc.load());
  });
}

/// the f16 matrix operators and functions, all the matCxR shapes
#[test]
fn f16_matrices() {
  check_compute(|builder| {
    let s = runtime_f16(builder);
    let v2 = s.splat::<Vec2<f16>>();
    let v3 = s.splat::<Vec3<f16>>();
    let v4 = s.splat::<Vec4<f16>>();

    let m2: Node<Mat2<f16>> = (v2, v2).into();
    let m3: Node<Mat3<f16>> = (v3, v3, v3).into();
    let m4: Node<Mat4<f16>> = (v4, v4, v4, v4).into();
    let m2x3: Node<Mat2x3<f16>> = (v3, v3).into();
    let m2x4: Node<Mat2x4<f16>> = (v4, v4).into();
    let m3x2: Node<Mat3x2<f16>> = (v2, v2, v2).into();
    let m3x4: Node<Mat3x4<f16>> = (v4, v4, v4).into();
    let m4x2: Node<Mat4x2<f16>> = (v2, v2, v2, v2).into();
    let m4x3: Node<Mat4x3<f16>> = (v3, v3, v3, v3).into();

    keep(m2 * v2 + v2 * m2);
    keep(m3 * m3 * s + s * m3 - m3);
    keep(m4 * v4);
    keep(m2x3 * v2 + m3 * v3);
    keep(m3x2 * m2x3);
    keep(m4x2 * m2x4);
    keep(m3x4 * v3 + m4x3.transpose() * v3);
    keep(m2.determinant() + m3.determinant() + m4.determinant());
    keep(m4.position() + m4.forward() + m4.scale());
    keep(m4.transpose());
  });
}

/// the f16 float built-in functions
#[test]
fn f16_builtin_functions() {
  check_compute(|builder| {
    let s = runtime_f16(builder);
    let v3 = s.splat::<Vec3<f16>>();

    keep(s.sqrt() + s.inverse_sqrt() + s.sin() + s.cos() + s.tan());
    keep(s.asin() + s.acos() + s.atan() + s.atan2(s));
    keep(s.sinh() + s.cosh() + s.tanh() + s.asinh() + s.acosh() + s.atanh());
    keep(v3.exp() + v3.exp2() + v3.ln() + v3.log2() + v3.pow(v3));
    keep(v3.floor() + v3.ceil() + v3.round() + v3.fract() + v3.trunc());
    keep(s.saturate() + s.degrees() + s.radians() + s.fma(s, s));
    keep(s.smoothstep(h(0.), h(1.)) + s.step(val_f16(0.5)));
    keep(v3.smoothstep_per_channel(v3, v3) + v3.mix_per_channel(v3, v3));
    keep(s.mix(v3, v3));
    keep(v3.abs() + v3.sign() + v3.min(v3).max(v3).clamp(v3, v3));
    keep(v3.normalize() + v3.cross(v3) + v3.reflect(v3) + v3.refract(v3, s));
    keep(v3.face_forward(v3, v3));
    keep(v3.length() + v3.distance(v3) + v3.dot(v3));
    keep(v3.max_channel() + v3.min_channel());
    keep(s.is_nan().or(s.is_inf()));

    let (fract, exp): (_, Node<Vec3<i32>>) = v3.frexp();
    keep(fract.ldexp(exp));
    let (fract, whole) = s.modf();
    keep(fract + whole);
  });
}

/// the conversions between f16 and the other scalars, and the f16 packing
#[test]
fn f16_conversions() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    let b = u.equals(val(0));
    let s = f.into_f16();
    let v4 = f.splat::<Vec4<f32>>().into_f16();

    keep(s.into_f32() + u.into_f16().into_f32() + i.into_f16().into_f32());
    keep(s.into_u32() + b.into_f16().into_u32());
    keep(s.into_i32());
    keep(s.into_bool());
    keep(v4.into_f32() + v4.into_u32().into_f16().into_f32());
    keep(v4.xy().pack2x_f16().unpack2x_f16());
  });
}

/// the f16 constants of all the primitive types, and the struct constant contains f16
#[test]
fn f16_constants() {
  check_compute(|builder| {
    let s = runtime_f16(builder);
    keep(val_f16(1.5) + val(h(-2.)) + s);
    keep(val(Vec3::new(h(1.), h(2.), h(3.))) + s);
    keep(val(Vec4::new(h(0.25), h(0.5), h(1.), h(2.))) * s);
    keep(val(Mat2::from([h(1.), h(2.), h(3.), h(4.)])) * s);
    keep(zeroed_val::<Mat4x3<f16>>());
    keep(val(F16Internal {
      a: h(1.),
      v3: Vec3::new(h(2.), h(2.), h(2.)),
      b: 3.,
      v2: Vec2::new(h(4.), h(4.)),
    }));
  });
}

/// The shader internal struct, the layout is determined by the shader side.
#[shader_struct]
#[derive(Clone, Copy)]
pub struct F16Internal {
  pub a: f16,
  pub v3: Vec3<f16>,
  pub b: f32,
  pub v2: Vec2<f16>,
}

/// The f16 fields in the std430 struct, the vec3 is followed by the scalar, and the matrices whose
/// rust column stride matches WGSL.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Std430F16 {
  pub a: f16,
  pub v3: Vec3<f16>,
  pub b: f16,
  pub v2: Vec2<f16>,
  pub c: f32,
  pub v4: Vec4<f16>,
  pub d: f16,
  pub m2: Mat2<f16>,
  pub m3x2: Mat3x2<f16>,
  pub e: f16,
  pub m4: Mat4<f16>,
  pub m2x4: Mat2x4<f16>,
  pub f: f16,
}

/// The std140 inner struct is aligned to 16 and its size is rounded up to 16, the natural layout
/// is not, so the padding contains f16.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Std140F16Inner {
  pub x: f16,
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Std140F16 {
  pub a: f16,
  pub inner: Std140F16Inner,
  pub v3: Vec3<f16>,
  pub b: f16,
  pub v4: Vec4<f16>,
  pub c: f16,
  pub arr: Shader140Array<Std140F16Inner, 2>,
  pub d: f16,
}

/// the host layout of the f16 struct must be the natural WGSL layout, the naga backend asserts it
/// when building
#[test]
fn f16_host_layout() {
  assert_eq!(size_of::<Std430F16>(), 120);
  assert_eq!(Std430F16::ALIGNMENT, 8);
  assert_eq!(std::mem::offset_of!(Std430F16, v3), 8);
  assert_eq!(std::mem::offset_of!(Std430F16, b), 14);
  assert_eq!(std::mem::offset_of!(Std430F16, m4), 64);

  assert_eq!(size_of::<Std140F16>(), 112);
  assert_eq!(std::mem::offset_of!(Std140F16, inner), 16);
  assert_eq!(std::mem::offset_of!(Std140F16, v3), 32);
  assert_eq!(std::mem::offset_of!(Std140F16, b), 38);
  assert_eq!(std::mem::offset_of!(Std140F16, arr), 64);

  check_compute(|builder| {
    let s = runtime_f16(builder);
    keep(zeroed_val::<Std430F16>());
    let padded = ENode::<Std140F16> {
      a: s,
      ..zeroed_val::<Std140F16>().expand()
    };
    keep(padded.construct());
  });
}

/// the f16 in the buffer address spaces and the workgroup and private variables
#[test]
fn f16_buffer_and_variables() {
  use BufferSpace::*;
  check_compute(|builder| {
    let RuntimeValues { u, .. } = runtime_values(builder);
    let s = runtime_f16(builder);

    let uniform = typed_readonly_ptr::<Std140F16>(&fake_buffer::<Std140F16>(0, Uniform));
    keep(uniform.inner().x().load() + uniform.v3().load().x());
    keep(uniform.arr().index(u).load());
    keep(uniform.load());

    let storage = typed_ptr::<[Std430F16]>(&fake_buffer::<[Std430F16]>(1, ReadWriteStorage));
    let item = storage.index(u);
    item.m4().index(u).z().store(s);
    item.v3().y().store(s);
    item.store(item.load());

    let scalars = typed_ptr::<[f16]>(&fake_buffer::<[f16]>(2, ReadWriteStorage));
    scalars.index(u).store(s + scalars.index(u + val(1)).load());

    let shared = builder.define_workgroup_shared_var::<[Vec4<f16>; 8]>();
    shared.index(u).store(s.splat());
    let private = private_var::<Std430F16>();
    private.d().store(s);
    let local = make_local_var::<[f16; 4]>();
    local.index(u).store(private.d().load());
    keep(local.load());
  });
}

/// the f16 subgroup operations
#[test]
fn f16_subgroup() {
  check_compute(|builder| {
    let v = runtime_f16(builder).splat::<Vec2<f16>>();
    keep(v.subgroup_add() + v.subgroup_inclusive_mul() + v.subgroup_max());
    keep(v.subgroup_broadcast_first() + v.subgroup_shuffle_xor(val(1)));
  });
}

/// the u32 heap maps each scalar to one u32, the f16 can not be represented
#[test]
#[should_panic(expected = "f16 is not supported in the u32 heap")]
fn f16_in_u32_heap() {
  check_compute(|_| {
    let heap = fake_storage_buffer::<[u32]>(0);
    keep(u32_heap_ptr::<Std430F16>(heap, 0).a().load());
  });
}

/// the packed layout is the layout of the u32 serialization
#[test]
#[should_panic(expected = "f16 is not supported in the packed layout")]
fn f16_in_packed_layout() {
  Vec2::<f16>::sized_ty().size_of_self(StructLayoutTarget::Packed);
}

/// The f16 arithmetic on the GPU, the results are stored as f16: the products are exact, the large
/// product overflows the f16 range, and the sums are rounded to the f16 precision (the spacing near
/// 2048 is 2).
///
/// The intermediate values are not checked by converting to f32 in shader, the Metal compiler is in
/// the fast math mode by default and may evaluate the f16 expression in f32 without rounding.
#[pollster::test]
async fn f16_arithmetic_gpu() {
  if !gpu_supports(Features::SHADER_F16).await {
    return;
  }
  let input = [0.75, 1.25, 3.5, -4.0, 1000.0];
  let result = gpu_map(&input, |x: Node<f32>| {
    let s = x.into_f16();
    (s * s, s + val_f16(2048.), -s, s / val_f16(4.)).into()
  })
  .await;

  let expect: Vec<_> = input
    .iter()
    .map(|x| {
      let s = h(*x);
      Vec4::new(s * s, s + h(2048.), -s, s / h(4.))
    })
    .collect();
  assert_eq!(result, expect);
  assert_eq!(result[1].y, h(2050.));
  assert!(result[4].x.is_infinite());
}

fn std430_f16_data(seed: f32) -> Std430F16 {
  let mut data = Std430F16::zeroed();
  let mut next = seed;
  let mut value = || {
    next += 1.;
    h(next)
  };
  data.a = value();
  data.v3 = Vec3::new(value(), value(), value());
  data.b = value();
  data.v2 = Vec2::new(value(), value());
  data.c = value().to_f32();
  data.v4 = Vec4::new(value(), value(), value(), value());
  data.d = value();
  let m2: [f16; 4] = std::array::from_fn(|_| value());
  data.m2 = m2.into();
  let m3x2: [f16; 6] = std::array::from_fn(|_| value());
  data.m3x2 = m3x2.into();
  data.e = value();
  let m4: [f16; 16] = std::array::from_fn(|_| value());
  data.m4 = m4.into();
  let m2x4: [f16; 8] = std::array::from_fn(|_| value());
  data.m2x4 = m2x4.into();
  data.f = value();
  data
}

/// the f16 struct in the storage buffer is loaded, modified by the f16 arithmetic and stored, so
/// the host layout matches the GPU one
#[pollster::test]
async fn f16_storage_struct_gpu() {
  if !gpu_supports(Features::SHADER_F16).await {
    return;
  }
  let input = [std430_f16_data(0.), std430_f16_data(100.)];
  let result = gpu_map(&input, |s: Node<Std430F16>| {
    let s = s.expand();
    let two = val_f16(2.);
    ENode::<Std430F16> {
      a: s.a * two,
      v3: s.v3 * two,
      b: s.b * two,
      v2: s.v2 * two,
      c: s.c * val(2.),
      v4: s.v4 * two,
      d: s.d * two,
      m2: s.m2 * two,
      m3x2: s.m3x2 * two,
      e: s.e * two,
      m4: s.m4 * two,
      m2x4: s.m2x4 * two,
      f: s.f * two,
    }
    .construct()
  })
  .await;

  let expect: Vec<_> = input
    .iter()
    .map(|s| {
      let two = h(2.);
      let mut r = *s;
      r.a = s.a * two;
      r.v3 = double(s.v3);
      r.b = s.b * two;
      r.v2 = double(s.v2);
      r.c = s.c * 2.;
      r.v4 = double(s.v4);
      r.d = s.d * two;
      r.m2 = double(s.m2);
      r.m3x2 = double(s.m3x2);
      r.e = s.e * two;
      r.m4 = double(s.m4);
      r.m2x4 = double(s.m2x4);
      r.f = s.f * two;
      r
    })
    .collect();
  assert_eq!(result, expect);
}

fn double<V: Into<[f16; N]> + From<[f16; N]>, const N: usize>(v: V) -> V {
  let components: [f16; N] = v.into();
  components.map(|c| c * h(2.)).into()
}

fn std140_f16_data() -> Std140F16 {
  let mut data = Std140F16::zeroed();
  data.a = h(1.);
  data.inner.x = h(2.);
  data.v3 = Vec3::new(h(3.), h(4.), h(5.));
  data.b = h(6.);
  data.v4 = Vec4::new(h(7.), h(8.), h(9.), h(10.));
  data.c = h(11.);
  data.arr.inner[0].inner.x = h(12.);
  data.arr.inner[1].inner.x = h(13.);
  data.d = h(14.);
  data
}

fn std140_f16_leaves(
  builder: &ShaderComputePipelineBuilder,
  ptrs: &[BoxedShaderPtr],
) -> Vec<Node<f32>> {
  let index = builder.local_invocation_index();
  let s = typed_readonly_ptr::<Std140F16>(&ptrs[0]);
  let v3 = s.v3().load();
  let v4 = s.v4().load();
  let loaded = s.load().expand();
  [
    s.a().load(),
    s.inner().x().load(),
    v3.x(),
    v3.y(),
    v3.z(),
    s.b().load(),
    v4.x(),
    v4.y(),
    v4.z(),
    v4.w(),
    s.c().load(),
    s.arr().index(index).x().load(),
    s.arr().index(index + val(1)).x().load(),
    loaded.d,
  ]
  .map(|v| v.into_f32())
  .to_vec()
}

fn std140_f16_buffers() -> [RawBuffer; 1] {
  [RawBuffer {
    ty: Std140F16::ty(),
    space: BufferSpace::Uniform,
    bytes: bytes_of(&std140_f16_data()).to_vec(),
  }]
}

/// the std140 struct with the f16 padding in the uniform buffer
#[test]
fn f16_uniform_struct() {
  check_raw_buffers(&std140_f16_buffers(), std140_f16_leaves);
}

/// the GPU version of [f16_uniform_struct]
#[pollster::test]
async fn f16_uniform_struct_gpu() {
  if !gpu_supports(Features::SHADER_F16).await {
    return;
  }
  let buffers = std140_f16_buffers();
  let result = gpu_run_raw_buffers(&buffers, 1, 14, std140_f16_leaves).await;
  let expect: Vec<_> = (1..=14).map(|v| v as f32).collect();
  assert_eq!(result.output[0], expect);
}

both!(IOF16, f16);
both!(IOVec2F16, Vec2<f16>);
both!(IOVec4F16, Vec4<f16>);

only_vertex!(IOF16Bias, f16);

/// The f16 vertex, the array stride is 16, the attribute offsets are aligned to `min(4, size)`.
#[repr(C)]
#[derive(Clone, Copy, ShaderVertex)]
pub struct F16Vertex {
  #[semantic(IOVec2F16)]
  pub uv: Vec2<f16>,
  #[semantic(IOVec4F16)]
  pub color: Vec4<f16>,
  #[semantic(IOF16)]
  pub scale: f16,
  #[semantic(IOF16Bias)]
  pub bias: f16,
}

fn f16_io_logic(builder: &mut ShaderRenderPipelineBuilder) {
  builder.vertex(|builder, _| {
    let vertex = builder.expect_vertex_shader();
    vertex.register_vertex::<F16Vertex>(VertexStepMode::Vertex);

    let uv = builder.query::<IOVec2F16>();
    let scale = builder.query::<IOF16>() + builder.query::<IOF16Bias>();
    let position = builder.query::<IOVec4F16>().into_f32();
    builder.register::<ClipPosition>(position);
    builder.set_vertex_out::<IOVec2F16>(uv * scale);
    builder.set_vertex_out_with_given_interpolate::<IOF16>(scale, ShaderInterpolation::Flat);
  });
  builder.fragment(|builder, _| {
    let uv = builder.query::<IOVec2F16>().into_f32();
    let scale = builder.query::<IOF16>().into_f32();
    builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
    builder.store_fragment_out_vec4f(0, (uv, scale, val(1.)));
  });
}

/// the single f16 attribute makes the array stride 2, which is not a multiple of 4
#[test]
#[should_panic(expected = "InvalidVertexBufferLayout")]
fn f16_invalid_vertex_stride() {
  build_graphics_pipeline(
    |builder| {
      builder.vertex(|builder, _| {
        let vertex = builder.expect_vertex_shader();
        vertex.push_single_vertex_layout::<IOF16>(VertexStepMode::Vertex);
        let scale = builder.query::<IOF16>();
        builder.register::<ClipPosition>(scale.into_f32().splat());
      });
    },
    None,
    &naga_stage_api,
  );
}

/// the f16 vertex inputs use the float16 vertex formats, and the f16 inter stage variables are
/// interpolated like f32
#[test]
fn f16_vertex_input_and_inter_stage_io() {
  let result = build_graphics_pipeline(f16_io_logic, None, &naga_stage_api);

  let attributes: Vec<_> = result.vertex_layouts[0]
    .attributes
    .iter()
    .map(|a| (a.format, a.offset, a.shader_location))
    .collect();
  use VertexFormat::*;
  assert_eq!(result.vertex_layouts[0].array_stride, 16);
  assert_eq!(
    attributes,
    [
      (Float16x2, 0, 0),
      (Float16x4, 4, 1),
      (Float16, 12, 2),
      (Float16, 14, 3)
    ]
  );

  let VertexOrTaskMesh::Vertex(vertex) = result.shape_shader else {
    unreachable!("expect vertex shader")
  };
  let vertex = naga_module(vertex);
  let fragment = naga_module(result.frag_shader);
  validate(&vertex);
  validate(&fragment);

  let f16 = naga::Scalar::F16;
  let outputs = location_members(&vertex, entry_result_members(&vertex));
  let outputs: Vec<_> = outputs
    .iter()
    .map(|io| (io.location, io.ty.clone(), io.interpolation))
    .collect();
  assert_eq!(
    outputs,
    [
      (
        0,
        naga::TypeInner::Vector {
          size: naga::VectorSize::Bi,
          scalar: f16,
        },
        Some(naga::Interpolation::Perspective)
      ),
      (
        1,
        naga::TypeInner::Scalar(f16),
        Some(naga::Interpolation::Flat)
      ),
    ]
  );
  let inputs: Vec<_> = location_arguments(&fragment)
    .iter()
    .map(|io| (io.location, io.ty.clone(), io.interpolation))
    .collect();
  assert_eq!(inputs, outputs);
}

/// the render pipeline of [f16_io_logic] is created by the device, so the vertex formats and the
/// inter stage variables are checked by wgpu and the platform shader compiler
#[pollster::test]
async fn f16_vertex_input_and_inter_stage_io_gpu() {
  if !gpu_supports(Features::SHADER_F16).await {
    return;
  }
  struct Logic;
  impl GraphicsShaderProvider for Logic {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
      f16_io_logic(builder)
    }
  }

  let gpu = rendiation_webgpu::GPU::new(Default::default())
    .await
    .unwrap();
  let scope = gpu
    .device
    .push_error_scope(rendiation_webgpu::raw_gpu::ErrorFilter::Validation);
  let builder = Logic
    .build_self(
      &naga_stage_api,
      None,
      gpu.info.clone(),
      ShaderRuntimeChecks::default(),
    )
    .unwrap_or_else(|e| panic!("failed to build shader: {e:?}"));
  let pipeline = gpu.device.build_pipeline_by_shader_api(builder, "f16 io");
  assert!(pipeline.is_ok());
  if let Some(error) = scope.pop().await {
    panic!("failed to create the f16 io pipeline: {error}");
  }
}
