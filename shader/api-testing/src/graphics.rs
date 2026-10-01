use std::any::Any;
use std::sync::Arc;

use rendiation_shader_api::*;
use rendiation_shader_backend_naga::*;

use crate::harness::*;

both!(IOFloat, f32);
both!(IOVec3F32, Vec3<f32>);
both!(IOVec4U32, Vec4<u32>);
both!(IOVec2I32, Vec2<i32>);
both!(IOU32, u32);
both!(IOVec2F32, Vec2<f32>);
both!(IOVec4F32, Vec4<f32>);
only_vertex!(InstanceTransform, Mat4<f32>);
only_vertex!(LateUvSource, Vec2<f32>);
only_fragment!(LateUv, Vec2<f32>);

/// Build the graphics pipeline by the stage `api`, the mesh shading logic replaces the vertex
/// stage by the task and mesh stages. Unlike `build_graphics`, the whole result is returned.
pub fn build_graphics_pipeline(
  logic: impl Fn(&mut ShaderRenderPipelineBuilder),
  mesh_shading: Option<&dyn MeshShaderLogic>,
  api: &dyn Fn(ShaderStage) -> DynamicShaderAPI,
) -> GraphicsShaderCompileResult {
  struct Logic<F>(F);
  impl<F: Fn(&mut ShaderRenderPipelineBuilder)> GraphicsShaderProvider for Logic<F> {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
      (self.0)(builder)
    }
  }

  Logic(logic)
    .build_self(
      api,
      mesh_shading,
      Arc::new(all_features_gpu_info()),
      ShaderRuntimeChecks::default(),
    )
    .unwrap_or_else(|e| panic!("failed to build shader: {e:?}"))
    .build()
    .expect("failed to build shader")
}

/// The stage api of the naga backend.
pub fn naga_stage_api(stage: ShaderStage) -> DynamicShaderAPI {
  Box::new(ShaderAPINagaImpl::new(stage))
}

/// The naga module of a built shader stage.
pub fn naga_module((_, shader): (String, Box<dyn Any>)) -> naga::Module {
  shader
    .downcast::<NagaModuleBuildResult>()
    .expect("expect naga backend build result")
    .module
}

fn all_features_gpu_info() -> GPUInfo {
  GPUInfo {
    adaptor_info: wgpu_types::AdapterInfo {
      name: String::new(),
      vendor: 0,
      device: 0,
      device_type: wgpu_types::DeviceType::Other,
      device_pci_bus_id: String::new(),
      driver: String::new(),
      driver_info: String::new(),
      backend: wgpu_types::Backend::Noop,
      subgroup_min_size: 4,
      subgroup_max_size: 128,
      transient_saves_memory: false,
    },
    power_preference: Default::default(),
    supported_features: wgpu_types::Features::all(),
    supported_limits: Default::default(),
    downgrade_info: Default::default(),
  }
}

/// The members of the struct returned by the entry point, empty if it returns nothing.
pub fn entry_result_members(module: &naga::Module) -> &[naga::StructMember] {
  let Some(result) = &module.entry_points[0].function.result else {
    return &[];
  };
  match &module.types[result.ty].inner {
    naga::TypeInner::Struct { members, .. } => members,
    other => panic!("expect struct entry result, got {other:?}"),
  }
}

/// The user defined IO, the location is the `@location` attribute.
#[derive(Debug, PartialEq)]
pub struct LocationIO {
  pub location: u32,
  pub ty: naga::TypeInner,
  pub interpolation: Option<naga::Interpolation>,
  pub sampling: Option<naga::Sampling>,
}

fn location_io<'a>(
  module: &naga::Module,
  io: impl Iterator<Item = (Option<&'a naga::Binding>, naga::Handle<naga::Type>)>,
) -> Vec<LocationIO> {
  let mut io: Vec<_> = io
    .filter_map(|(binding, ty)| match binding? {
      naga::Binding::Location {
        location,
        interpolation,
        sampling,
        ..
      } => Some(LocationIO {
        location: *location,
        ty: module.types[ty].inner.clone(),
        interpolation: *interpolation,
        sampling: *sampling,
      }),
      naga::Binding::BuiltIn(_) => None,
    })
    .collect();
  io.sort_by_key(|io| io.location);
  io
}

/// The user defined IO of the struct members, sorted by location.
pub fn location_members(module: &naga::Module, members: &[naga::StructMember]) -> Vec<LocationIO> {
  location_io(module, members.iter().map(|m| (m.binding.as_ref(), m.ty)))
}

/// The user defined inputs of the entry point, sorted by location.
pub fn location_arguments(module: &naga::Module) -> Vec<LocationIO> {
  let arguments = &module.entry_points[0].function.arguments;
  location_io(module, arguments.iter().map(|a| (a.binding.as_ref(), a.ty)))
}

/// The built-in IO of the struct members, in the member order.
pub fn builtin_members(members: &[naga::StructMember]) -> Vec<naga::BuiltIn> {
  members
    .iter()
    .filter_map(|m| match m.binding {
      Some(naga::Binding::BuiltIn(b)) => Some(b),
      _ => None,
    })
    .collect()
}

/// The built-in inputs of the entry point, in the argument order.
pub fn builtin_arguments(module: &naga::Module) -> Vec<naga::BuiltIn> {
  module.entry_points[0]
    .function
    .arguments
    .iter()
    .filter_map(|a| match a.binding {
      Some(naga::Binding::BuiltIn(b)) => Some(b),
      _ => None,
    })
    .collect()
}

/// The type of the struct member that has the built-in binding.
pub fn builtin_member_ty<'a>(
  module: &'a naga::Module,
  members: &[naga::StructMember],
  builtin: naga::BuiltIn,
) -> &'a naga::TypeInner {
  let member = members
    .iter()
    .find(|m| m.binding == Some(naga::Binding::BuiltIn(builtin)))
    .unwrap_or_else(|| panic!("expect built-in member {builtin:?}"));
  &module.types[member.ty].inner
}

fn contains_kill(block: &naga::Block) -> bool {
  block.iter().any(|s| match s {
    naga::Statement::Kill => true,
    naga::Statement::Block(b) => contains_kill(b),
    naga::Statement::If { accept, reject, .. } => contains_kill(accept) || contains_kill(reject),
    naga::Statement::Loop {
      body, continuing, ..
    } => contains_kill(body) || contains_kill(continuing),
    naga::Statement::Switch { cases, .. } => cases.iter().any(|c| contains_kill(&c.body)),
    _ => false,
  })
}

const F32: naga::TypeInner = naga::TypeInner::Scalar(naga::Scalar::F32);
const U32: naga::TypeInner = naga::TypeInner::Scalar(naga::Scalar::U32);

const fn vector(size: naga::VectorSize, scalar: naga::Scalar) -> naga::TypeInner {
  naga::TypeInner::Vector { size, scalar }
}

/// the user defined inter stage IO with numeric scalar and vector, the integer types are flat
/// interpolated
#[test]
fn user_defined_io() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      builder
        .expect_vertex_shader()
        .push_single_vertex_layout::<IOVec3F32>(VertexStepMode::Vertex);
      let position = builder.query::<IOVec3F32>();
      let index = builder.query::<VertexIndex>();

      builder.set_vertex_out::<IOFloat>(index.into_f32());
      builder.set_vertex_out::<IOVec3F32>(position);
      builder.set_vertex_out::<IOVec4U32>(index.splat::<Vec4<u32>>());
      builder.set_vertex_out_with_given_interpolate::<IOVec2I32>(
        index.into_i32().splat::<Vec2<i32>>(),
        ShaderInterpolation::Flat,
      );
    });
    builder.fragment(|builder, _| {
      let f = builder.query::<IOFloat>();
      let v = builder.query::<IOVec3F32>();
      let u = builder.query::<IOVec4U32>();
      let i = builder.query::<IOVec2I32>();
      keep(f.splat::<Vec3<f32>>() + v);
      keep(u.x() + i.x().into_u32());
    });
  });
}

/// the built-in values: clip distances, primitive index, and the fragment subgroup values
#[test]
fn builtin_values() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>().into_f32();
      let distances = make_local_var::<[f32; 2]>();
      distances.index(0).store(index);
      distances.index(1).store(-index);
      builder
        .expect_vertex_shader()
        .set_clip_distances(distances.load());
    });
    builder.fragment(|builder, _| {
      let index = builder.query::<FragmentPrimitiveIndex>();
      let size = builder.query::<FragmentSubgroupSize>();
      let id = builder.query::<FragmentSubgroupInvocationId>();
      keep(index + size + id);
    });
  });
}

/// several vertex buffers of both step modes, the attribute locations are assigned in the
/// registration order across the buffers and match the vertex entry arguments
#[test]
fn vertex_buffers_and_step_modes() {
  let result = build_graphics_pipeline(
    |builder| {
      builder.vertex(|builder, _| {
        let vertex = builder.expect_vertex_shader();
        let mut attributes = AttributesListBuilder::default();
        <Vec3<f32> as VertexInBuilder>::build_attribute::<IOVec3F32>(&mut attributes, vertex);
        <Vec2<f32> as VertexInBuilder>::build_attribute::<IOVec2F32>(&mut attributes, vertex);
        <u32 as VertexInBuilder>::build_attribute::<IOU32>(&mut attributes, vertex);
        attributes.build(vertex, VertexStepMode::Vertex);
        // the matrix is split into four vec4 attributes
        vertex.push_single_vertex_layout::<InstanceTransform>(VertexStepMode::Instance);
        vertex.push_single_vertex_layout::<IOFloat>(VertexStepMode::Instance);

        let position = builder.query::<IOVec3F32>();
        let transform = builder.query::<InstanceTransform>();
        let scale = builder.query::<IOFloat>();
        builder.register::<ClipPosition>(transform * (position * scale, val(1.)).into());
        let uv = builder.query::<IOVec2F32>();
        builder.set_vertex_out::<IOVec2F32>(uv);
        let id = builder.query::<IOU32>() + builder.query::<VertexInstanceIndex>();
        builder.set_vertex_out::<IOU32>(id);
      });
      builder.fragment(|builder, _| {
        keep(builder.query::<IOVec2F32>());
        keep(builder.query::<IOU32>());
      });
    },
    None,
    &naga_stage_api,
  );

  let layouts: Vec<_> = result
    .vertex_layouts
    .iter()
    .map(|layout| {
      let attributes: Vec<_> = layout
        .attributes
        .iter()
        .map(|a| (a.format, a.offset, a.shader_location))
        .collect();
      (layout.step_mode, layout.array_stride, attributes)
    })
    .collect();
  use VertexFormat::*;
  assert_eq!(
    layouts,
    [
      (
        VertexStepMode::Vertex,
        24,
        vec![(Float32x3, 0, 0), (Float32x2, 12, 1), (Uint32, 20, 2)]
      ),
      (
        VertexStepMode::Instance,
        64,
        vec![
          (Float32x4, 0, 3),
          (Float32x4, 16, 4),
          (Float32x4, 32, 5),
          (Float32x4, 48, 6)
        ]
      ),
      (VertexStepMode::Instance, 4, vec![(Float32, 0, 7)]),
    ]
  );

  let VertexOrTaskMesh::Vertex(vertex) = result.shape_shader else {
    unreachable!("expect vertex shader")
  };
  let vertex = naga_module(vertex);
  let fragment = naga_module(result.frag_shader);
  validate(&vertex);
  validate(&fragment);

  use naga::VectorSize::*;
  let inputs: Vec<_> = location_arguments(&vertex)
    .into_iter()
    .map(|io| (io.location, io.ty, io.interpolation))
    .collect();
  let vec4 = vector(Quad, naga::Scalar::F32);
  assert_eq!(
    inputs,
    [
      (0, vector(Tri, naga::Scalar::F32), None),
      (1, vector(Bi, naga::Scalar::F32), None),
      (2, U32, None),
      (3, vec4.clone(), None),
      (4, vec4.clone(), None),
      (5, vec4.clone(), None),
      (6, vec4, None),
      (7, F32, None),
    ]
  );
  assert_eq!(builtin_arguments(&vertex), [naga::BuiltIn::InstanceIndex]);
}

/// the vertex built-in inputs are created once, and the position output is invariant when
/// marked
#[test]
fn vertex_builtins_and_invariant_position() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.vertex(|builder, _| {
      let vertex_index = builder.query::<VertexIndex>();
      let instance_index = builder.query::<VertexInstanceIndex>();
      let instance_index = instance_index + builder.query::<VertexInstanceIndex>();
      builder.mark_position_invariant();
      let position = (vertex_index + instance_index).into_f32();
      builder.register::<ClipPosition>(position.splat::<Vec4<f32>>());
    });
  });
  validate(&vertex);
  validate(&fragment);

  assert_eq!(
    builtin_arguments(&vertex),
    [naga::BuiltIn::VertexIndex, naga::BuiltIn::InstanceIndex]
  );
  let members = entry_result_members(&vertex);
  let position = naga::BuiltIn::Position { invariant: true };
  assert_eq!(builtin_members(members), [position]);
  assert_eq!(
    builtin_member_ty(&vertex, members, position),
    &vector(naga::VectorSize::Quad, naga::Scalar::F32)
  );
}

/// the clip distances output is a built-in f32 array, the last set value overrides the previous
/// one, the position output is not invariant by default
#[test]
fn clip_distances_output() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>().into_f32();
      let single = make_local_var::<[f32; 1]>();
      single.index(0).store(index);
      let vertex = builder.expect_vertex_shader();
      vertex.set_clip_distances(single.load());
      vertex.set_clip_distances(zeroed_val::<[f32; 8]>());
    });
  });
  validate(&vertex);
  validate(&fragment);

  let members = entry_result_members(&vertex);
  assert_eq!(
    builtin_members(members),
    [
      naga::BuiltIn::Position { invariant: false },
      naga::BuiltIn::ClipDistance
    ]
  );
  let naga::TypeInner::Array { base, size, .. } =
    builtin_member_ty(&vertex, members, naga::BuiltIn::ClipDistance)
  else {
    panic!("expect clip distances array")
  };
  assert_eq!(vertex.types[*base].inner, F32);
  assert_eq!(
    *size,
    naga::ArraySize::Constant(std::num::NonZeroU32::new(8).unwrap())
  );
}

/// all the interpolation modes of the user defined IO are the same on both sides, perspective
/// by default, and the integer IO is always flat whatever is requested
#[test]
fn interpolation_modes() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>();
      let f = index.into_f32();
      builder.set_vertex_out::<IOFloat>(f);
      builder.set_vertex_out_with_given_interpolate::<IOVec2F32>(
        f.splat::<Vec2<f32>>(),
        ShaderInterpolation::Linear,
      );
      builder.set_vertex_out_with_given_interpolate::<IOVec3F32>(
        f.splat::<Vec3<f32>>(),
        ShaderInterpolation::Flat,
      );
      builder.set_vertex_out_with_given_interpolate::<IOVec4F32>(
        f.splat::<Vec4<f32>>(),
        ShaderInterpolation::Perspective,
      );
      builder.set_vertex_out_with_given_interpolate::<IOU32>(index, ShaderInterpolation::Linear);
    });
    builder.fragment(|builder, _| {
      keep(builder.query::<IOFloat>());
      keep(builder.query::<IOVec2F32>());
      keep(builder.query::<IOVec3F32>());
      keep(builder.query::<IOVec4F32>());
      keep(builder.query::<IOU32>());
    });
  });
  validate(&vertex);
  validate(&fragment);

  let outputs = location_members(&vertex, entry_result_members(&vertex));
  let interpolations: Vec<_> = outputs.iter().map(|io| io.interpolation).collect();
  use naga::Interpolation::*;
  assert_eq!(
    interpolations,
    [
      Some(Perspective),
      Some(Linear),
      Some(Flat),
      Some(Perspective),
      Some(Flat)
    ]
  );
  assert!(outputs.iter().all(|io| io.sampling.is_none()));
  assert_eq!(outputs, location_arguments(&fragment));
}

/// the vertex output locations are assigned in the declaration order, including the output
/// added by the fragment stage, and the fragment input locations, types and interpolations match
/// them even if the fragment stage only reads part of them
#[test]
fn inter_stage_locations() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>();
      builder.set_vertex_out::<IOVec4U32>(index.splat::<Vec4<u32>>());
      builder.set_vertex_out::<IOFloat>(index.into_f32());
      builder.register::<LateUvSource>(index.into_f32().splat::<Vec2<f32>>());
      builder.set_vertex_out::<IOVec2I32>(index.into_i32().splat::<Vec2<i32>>());
      // the repeated output takes no new location
      builder.set_vertex_out::<IOFloat>(val(1.));
    });
    builder.fragment(|builder, _| {
      keep(builder.query::<IOFloat>());
      keep(builder.query_or_interpolate_by::<LateUv, LateUvSource>());
    });
  });
  validate(&vertex);
  validate(&fragment);

  let outputs = location_members(&vertex, entry_result_members(&vertex));
  let types: Vec<_> = outputs
    .iter()
    .map(|io| (io.location, io.ty.clone()))
    .collect();
  use naga::VectorSize::*;
  assert_eq!(
    types,
    [
      (0, vector(Quad, naga::Scalar::U32)),
      (1, F32),
      (2, vector(Bi, naga::Scalar::I32)),
      (3, vector(Bi, naga::Scalar::F32)),
    ]
  );
  assert_eq!(outputs, location_arguments(&fragment));
}

/// several color targets of different output types, the output can be loaded and overwritten
#[test]
fn fragment_color_targets() {
  check_graphics(|builder| {
    builder.fragment(|builder, _| {
      let position = builder.query::<FragmentPosition>();
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.define_out_by(channel(TextureFormat::Rgba16Uint));
      builder.define_out_by(channel(TextureFormat::Rg32Sint));
      builder.define_out_by(channel(TextureFormat::R32Float));
      builder.define_out_by(channel(TextureFormat::R8Uint));
      builder.define_out_by(channel(TextureFormat::Rgba16Float).with_alpha_blend());

      builder.store_fragment_out_vec4f(0, position);
      builder.store_fragment_out(1, position.bitcast::<Vec4<u32>>());
      builder.store_fragment_out(2, position.xy().bitcast::<Vec2<i32>>());
      builder.store_fragment_out(3, position.x());
      builder.store_fragment_out(4, position.y().bitcast::<u32>());

      let loaded = builder.load_fragment_out::<Vec4<f32>>(0).unwrap();
      builder.store_fragment_out_vec4f(5, loaded * val(2.));
      if_by(loaded.x().greater_than(val(1.)), || {
        builder.store_fragment_out(3, loaded.y());
      });
      let port = &builder.frag_output[1];
      port.store(port.load::<Vec4<u32>>() + val(Vec4::one()));
    });
  });
}

/// the color outputs take the locations in the declaration order, and the depth output is a
/// built-in regardless of when it is registered
#[test]
fn fragment_output_locations() {
  for depth_first in [true, false] {
    let [vertex, fragment] = build_graphics(|builder| {
      builder.fragment(|builder, _| {
        let position = builder.query::<FragmentPosition>();
        if depth_first {
          builder.register::<FragmentDepthOutput>(position.z());
        }
        builder.define_out_by(channel(TextureFormat::Rgba16Uint));
        builder.define_out_by(channel(TextureFormat::R32Float));
        builder.define_out_by(channel(TextureFormat::Rg32Sint));
        if !depth_first {
          builder.register::<FragmentDepthOutput>(position.z());
        }
        builder.store_fragment_out(0, position.bitcast::<Vec4<u32>>());
        builder.store_fragment_out(1, position.w());
        builder.store_fragment_out(2, position.xy().bitcast::<Vec2<i32>>());
      });
    });
    validate(&vertex);
    validate(&fragment);

    let members = entry_result_members(&fragment);
    let outputs: Vec<_> = location_members(&fragment, members)
      .into_iter()
      .map(|io| (io.location, io.ty, io.interpolation))
      .collect();
    use naga::VectorSize::*;
    assert_eq!(
      outputs,
      [
        (0, vector(Quad, naga::Scalar::U32), None),
        (1, F32, None),
        (2, vector(Bi, naga::Scalar::I32), None),
      ]
    );
    assert_eq!(builtin_members(members), [naga::BuiltIn::FragDepth]);
    assert_eq!(
      builtin_member_ty(&fragment, members, naga::BuiltIn::FragDepth),
      &F32
    );
  }
}

/// the depth only pipeline: no color target, the depth output is the only output, and the early
/// depth test is not configured by default
#[test]
fn fragment_depth_only() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.fragment(|builder, _| {
      let position = builder.query::<FragmentPosition>();
      builder.register::<FragmentDepthOutput>(position.z() * val(0.5));
    });
  });
  validate(&vertex);
  validate(&fragment);

  let members = entry_result_members(&fragment);
  assert_eq!(members.len(), 1);
  assert_eq!(builtin_members(members), [naga::BuiltIn::FragDepth]);
  assert_eq!(fragment.entry_points[0].early_depth_test, None);
}

/// the fragment shader without any output returns nothing
#[test]
fn fragment_without_output() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.fragment(|builder, _| keep(builder.query::<FragmentPosition>()));
  });
  validate(&vertex);
  validate(&fragment);
  assert!(fragment.entry_points[0].function.result.is_none());
}

/// discard in a branch with the forced early depth test
#[test]
fn discard_with_forced_early_depth_test() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.fragment(|builder, _| {
      builder.set_early_depth_test(ShaderEarlyDepthTest::Force);
      let position = builder.query::<FragmentPosition>();
      if_by(position.x().less_than(val(1.)), || builder.discard());
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.store_fragment_out_vec4f(0, position);
    });
  });
  validate(&vertex);
  validate(&fragment);

  let entry = &fragment.entry_points[0];
  assert_eq!(entry.early_depth_test, Some(naga::EarlyDepthTest::Force));
  assert!(contains_kill(&entry.function.body));
}

/// the early depth test allowed with each conservative depth restriction of the depth output
#[test]
fn allowed_early_depth_test_with_depth_output() {
  use ShaderConservativeDepth::*;
  let cases = [
    (GreaterEqual, naga::ConservativeDepth::GreaterEqual),
    (LessEqual, naga::ConservativeDepth::LessEqual),
    (Unchanged, naga::ConservativeDepth::Unchanged),
  ];
  for (conservative, expect) in cases {
    let [vertex, fragment] = build_graphics(|builder| {
      builder.fragment(|builder, _| {
        builder.set_early_depth_test(ShaderEarlyDepthTest::Allow { conservative });
        let position = builder.query::<FragmentPosition>();
        builder.register::<FragmentDepthOutput>(position.z());
      });
    });
    validate(&vertex);
    validate(&fragment);
    assert_eq!(
      fragment.entry_points[0].early_depth_test,
      Some(naga::EarlyDepthTest::Allow {
        conservative: expect
      })
    );
  }
}

/// the fragment built-in inputs, each one is created once
#[test]
fn fragment_builtin_inputs() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.fragment(|builder, _| {
      for _ in 0..2 {
        let position = builder.query::<FragmentPosition>();
        let front_facing = builder.query::<FragmentFrontFacing>();
        let sample_index = builder.query::<FragmentSampleIndex>();
        let sample_mask = builder.query::<FragmentSampleMaskInput>();
        let primitive_index = builder.query::<FragmentPrimitiveIndex>();
        keep(front_facing.select(position, zeroed_val()));
        keep(sample_index + sample_mask + primitive_index);
      }
    });
  });
  validate(&vertex);
  validate(&fragment);

  use naga::BuiltIn::*;
  assert_eq!(
    builtin_arguments(&fragment),
    [
      Position { invariant: false },
      FrontFacing,
      SampleIndex,
      SampleMask,
      PrimitiveIndex
    ]
  );
}

/// the registered sample mask output is the built-in sample_mask output
#[test]
#[ignore = "bug: FragmentSampleMaskOutput is never written to the sample_mask output"]
fn fragment_sample_mask_output() {
  let [vertex, fragment] = build_graphics(|builder| {
    builder.fragment(|builder, _| {
      let mask = builder.query::<FragmentSampleMaskInput>();
      builder.register::<FragmentSampleMaskOutput>(mask & val(1));
    });
  });
  validate(&vertex);
  validate(&fragment);
  assert_eq!(
    builtin_members(entry_result_members(&fragment)),
    [naga::BuiltIn::SampleMask]
  );
}

/// the four channel 32 bit integer targets are written by the four component vectors
#[test]
#[ignore = "bug: the Rgba32Uint and Rgba32Sint targets map to the scalar output type"]
fn fragment_rgba32_integer_targets() {
  check_graphics(|builder| {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(TextureFormat::Rgba32Uint));
      builder.define_out_by(channel(TextureFormat::Rgba32Sint));
      builder.store_fragment_out(0, zeroed_val::<Vec4<u32>>());
      builder.store_fragment_out(1, zeroed_val::<Vec4<i32>>());
    });
  });
}

/// the stored value type must be the target output type
#[test]
#[should_panic(expected = "attachment expect ty")]
fn fragment_output_type_mismatch() {
  build_graphics(|builder| {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.store_fragment_out(0, zeroed_val::<Vec4<u32>>());
    });
  });
}

/// the fragment output must be declared before storing, it is not declared implicitly
#[test]
#[should_panic(expected = "FragmentOutputSlotNotDeclared")]
fn fragment_output_not_declared() {
  build_graphics(|builder| {
    builder.fragment(|builder, _| {
      builder.store_fragment_out_vec4f(0, zeroed_val());
    });
  });
}

/// loading the undeclared fragment output is an error
#[test]
fn fragment_load_not_declared_output() {
  check_graphics(|builder| {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      let result = builder.load_fragment_out::<Vec4<f32>>(1);
      assert!(matches!(
        result,
        Err(ShaderBuildError::FragmentOutputSlotNotDeclared)
      ));
      builder.store_fragment_out_vec4f(0, zeroed_val());
    });
  });
}

/// the semantic queried in the fragment stage must be provided
#[test]
#[should_panic(expected = "MissingRequiredDependency")]
fn fragment_missing_semantic() {
  build_graphics(|builder| {
    builder.fragment(|builder, _| keep(builder.query::<IOFloat>()));
  });
}
