use rendiation_shader_api::*;

pub fn map_stage(stage: ShaderStage) -> naga::ShaderStage {
  match stage {
    ShaderStage::Vertex => naga::ShaderStage::Vertex,
    ShaderStage::Fragment => naga::ShaderStage::Fragment,
    ShaderStage::Task => naga::ShaderStage::Task,
    ShaderStage::Mesh => naga::ShaderStage::Mesh,
    ShaderStage::Compute => naga::ShaderStage::Compute,
  }
}

pub fn map_barrier(scope: BarrierScope) -> naga::Barrier {
  match scope {
    BarrierScope::Storage => naga::Barrier::STORAGE,
    BarrierScope::WorkGroup => naga::Barrier::WORK_GROUP,
    BarrierScope::SubGroup => naga::Barrier::SUB_GROUP,
  }
}

pub fn map_address_space(space: AddressSpace) -> naga::AddressSpace {
  match space {
    AddressSpace::Function => naga::AddressSpace::Function,
    AddressSpace::Private => naga::AddressSpace::Private,
    AddressSpace::WorkGroup => naga::AddressSpace::WorkGroup,
    AddressSpace::Uniform => naga::AddressSpace::Uniform,
    AddressSpace::Storage { writeable } => naga::AddressSpace::Storage {
      access: if writeable {
        naga::StorageAccess::LOAD | naga::StorageAccess::STORE
      } else {
        naga::StorageAccess::LOAD
      },
    },
    AddressSpace::Handle => naga::AddressSpace::Handle,
  }
}

pub fn map_mesh_output_topology(t: MeshOutputTopology) -> naga::MeshOutputTopology {
  match t {
    MeshOutputTopology::Points => naga::MeshOutputTopology::Points,
    MeshOutputTopology::Lines => naga::MeshOutputTopology::Lines,
    MeshOutputTopology::Triangles => naga::MeshOutputTopology::Triangles,
  }
}

pub fn map_switch_value(case: SwitchCaseCondition) -> naga::SwitchValue {
  match case {
    SwitchCaseCondition::U32(v) => naga::SwitchValue::U32(v),
    SwitchCaseCondition::I32(v) => naga::SwitchValue::I32(v),
    SwitchCaseCondition::Default => naga::SwitchValue::Default,
  }
}

pub fn map_atomic_scalar(t: ShaderAtomicValueType) -> naga::Scalar {
  match t {
    ShaderAtomicValueType::I32 => naga::Scalar::I32,
    ShaderAtomicValueType::U32 => naga::Scalar::U32,
  }
}

pub fn map_image_dimension(dimension: TextureViewDimension) -> (naga::ImageDimension, bool) {
  match dimension {
    TextureViewDimension::D1 => (naga::ImageDimension::D1, false),
    TextureViewDimension::D2 => (naga::ImageDimension::D2, false),
    TextureViewDimension::D2Array => (naga::ImageDimension::D2, true),
    TextureViewDimension::Cube => (naga::ImageDimension::Cube, false),
    TextureViewDimension::CubeArray => (naga::ImageDimension::Cube, true),
    TextureViewDimension::D3 => (naga::ImageDimension::D3, false),
  }
}

pub fn map_texture_sample_type(
  sample_type: TextureSampleType,
  multi_sampled: bool,
) -> naga::ImageClass {
  match sample_type {
    TextureSampleType::Float { .. } => naga::ImageClass::Sampled {
      kind: naga::ScalarKind::Float,
      multi: multi_sampled,
    },
    TextureSampleType::Depth => naga::ImageClass::Depth {
      multi: multi_sampled,
    },
    TextureSampleType::Sint => naga::ImageClass::Sampled {
      kind: naga::ScalarKind::Sint,
      multi: multi_sampled,
    },
    TextureSampleType::Uint => naga::ImageClass::Sampled {
      kind: naga::ScalarKind::Uint,
      multi: multi_sampled,
    },
  }
}

pub fn map_storage_format(format: StorageFormat) -> naga::StorageFormat {
  match format {
    StorageFormat::R8Unorm => naga::StorageFormat::R8Unorm,
    StorageFormat::R8Snorm => naga::StorageFormat::R8Snorm,
    StorageFormat::R8Uint => naga::StorageFormat::R8Uint,
    StorageFormat::R8Sint => naga::StorageFormat::R8Sint,
    StorageFormat::R16Uint => naga::StorageFormat::R16Uint,
    StorageFormat::R16Sint => naga::StorageFormat::R16Sint,
    StorageFormat::R16Float => naga::StorageFormat::R16Float,
    StorageFormat::Rg8Unorm => naga::StorageFormat::Rg8Unorm,
    StorageFormat::Rg8Snorm => naga::StorageFormat::Rg8Snorm,
    StorageFormat::Rg8Uint => naga::StorageFormat::Rg8Uint,
    StorageFormat::Rg8Sint => naga::StorageFormat::Rg8Sint,
    StorageFormat::R32Uint => naga::StorageFormat::R32Uint,
    StorageFormat::R32Sint => naga::StorageFormat::R32Sint,
    StorageFormat::R32Float => naga::StorageFormat::R32Float,
    StorageFormat::Rg16Uint => naga::StorageFormat::Rg16Uint,
    StorageFormat::Rg16Sint => naga::StorageFormat::Rg16Sint,
    StorageFormat::Rg16Float => naga::StorageFormat::Rg16Float,
    StorageFormat::Rgba8Unorm => naga::StorageFormat::Rgba8Unorm,
    StorageFormat::Rgba8Snorm => naga::StorageFormat::Rgba8Snorm,
    StorageFormat::Rgba8Uint => naga::StorageFormat::Rgba8Uint,
    StorageFormat::Rgba8Sint => naga::StorageFormat::Rgba8Sint,
    StorageFormat::Bgra8Unorm => naga::StorageFormat::Bgra8Unorm,
    StorageFormat::Rgb10a2Uint => naga::StorageFormat::Rgb10a2Uint,
    StorageFormat::Rgb10a2Unorm => naga::StorageFormat::Rgb10a2Unorm,
    StorageFormat::Rg32Uint => naga::StorageFormat::Rg32Uint,
    StorageFormat::Rg32Sint => naga::StorageFormat::Rg32Sint,
    StorageFormat::Rg32Float => naga::StorageFormat::Rg32Float,
    StorageFormat::Rgba16Uint => naga::StorageFormat::Rgba16Uint,
    StorageFormat::Rgba16Sint => naga::StorageFormat::Rgba16Sint,
    StorageFormat::Rgba16Float => naga::StorageFormat::Rgba16Float,
    StorageFormat::Rgba32Uint => naga::StorageFormat::Rgba32Uint,
    StorageFormat::Rgba32Sint => naga::StorageFormat::Rgba32Sint,
    StorageFormat::Rgba32Float => naga::StorageFormat::Rgba32Float,
    StorageFormat::R16Unorm => naga::StorageFormat::R16Unorm,
    StorageFormat::R16Snorm => naga::StorageFormat::R16Snorm,
    StorageFormat::Rg16Unorm => naga::StorageFormat::Rg16Unorm,
    StorageFormat::Rg16Snorm => naga::StorageFormat::Rg16Snorm,
    StorageFormat::Rgba16Unorm => naga::StorageFormat::Rgba16Unorm,
    StorageFormat::Rgba16Snorm => naga::StorageFormat::Rgba16Snorm,
  }
}

pub fn map_storage_access(access: StorageTextureAccess) -> naga::StorageAccess {
  match access {
    StorageTextureAccess::Load => naga::StorageAccess::LOAD,
    StorageTextureAccess::Store => naga::StorageAccess::STORE,
    StorageTextureAccess::LoadStore => naga::StorageAccess::LOAD | naga::StorageAccess::STORE,
  }
}

pub fn map_scalar_kind(t: ScalarType) -> naga::ScalarKind {
  match t {
    ScalarType::U32 => naga::ScalarKind::Uint,
    ScalarType::I32 => naga::ScalarKind::Sint,
    ScalarType::F32 => naga::ScalarKind::Float,
    ScalarType::Bool => naga::ScalarKind::Bool,
  }
}

pub fn map_scalar_type(t: ScalarType) -> naga::Scalar {
  match t {
    ScalarType::F32 => naga::Scalar::F32,
    ScalarType::U32 => naga::Scalar::U32,
    ScalarType::I32 => naga::Scalar::I32,
    ScalarType::Bool => naga::Scalar::BOOL,
  }
}

pub fn map_vector_size(t: VectorSize) -> naga::VectorSize {
  match t {
    VectorSize::Bi => naga::VectorSize::Bi,
    VectorSize::Tri => naga::VectorSize::Tri,
    VectorSize::Quad => naga::VectorSize::Quad,
  }
}

pub fn map_primitive_type(t: PrimitiveShaderValueType) -> naga::TypeInner {
  match t {
    PrimitiveShaderValueType::Scalar(scalar) => naga::TypeInner::Scalar(map_scalar_type(scalar)),
    PrimitiveShaderValueType::Vector { size, scalar } => naga::TypeInner::Vector {
      size: map_vector_size(size),
      scalar: map_scalar_type(scalar),
    },
    PrimitiveShaderValueType::Matrix {
      columns,
      rows,
      scalar,
    } => naga::TypeInner::Matrix {
      columns: map_vector_size(columns),
      rows: map_vector_size(rows),
      scalar: map_scalar_type(scalar),
    },
  }
}

pub fn map_primitive_vec_size(t: PrimitiveShaderValueType) -> Option<naga::VectorSize> {
  match t {
    PrimitiveShaderValueType::Vector { size, .. } => Some(map_vector_size(size)),
    _ => None,
  }
}

pub fn map_interpolation(interpolation: ShaderInterpolation) -> naga::Interpolation {
  match interpolation {
    ShaderInterpolation::Perspective => naga::Interpolation::Perspective,
    ShaderInterpolation::Linear => naga::Interpolation::Linear,
    ShaderInterpolation::Flat => naga::Interpolation::Flat,
  }
}

pub fn map_built_in(bt: ShaderBuiltInDecorator) -> naga::BuiltIn {
  match bt {
    ShaderBuiltInDecorator::VertexIndex => naga::BuiltIn::VertexIndex,
    ShaderBuiltInDecorator::VertexInstanceIndex => naga::BuiltIn::InstanceIndex,
    ShaderBuiltInDecorator::FragFrontFacing => naga::BuiltIn::FrontFacing,
    ShaderBuiltInDecorator::FragSampleIndex => naga::BuiltIn::SampleIndex,
    ShaderBuiltInDecorator::FragSampleMask => naga::BuiltIn::SampleMask,
    ShaderBuiltInDecorator::FragPositionIn => naga::BuiltIn::Position { invariant: false },
    ShaderBuiltInDecorator::VertexPositionOut => naga::BuiltIn::Position { invariant: false },
    ShaderBuiltInDecorator::FragDepth => naga::BuiltIn::FragDepth,
    ShaderBuiltInDecorator::CompLocalInvocationId => naga::BuiltIn::LocalInvocationId,
    ShaderBuiltInDecorator::CompGlobalInvocationId => naga::BuiltIn::GlobalInvocationId,
    ShaderBuiltInDecorator::CompLocalInvocationIndex => naga::BuiltIn::LocalInvocationIndex,
    ShaderBuiltInDecorator::CompWorkgroupId => naga::BuiltIn::WorkGroupId,
    ShaderBuiltInDecorator::CompNumWorkgroup => naga::BuiltIn::NumWorkGroups,
    ShaderBuiltInDecorator::CompSubgroupInvocationId => naga::BuiltIn::SubgroupInvocationId,
    ShaderBuiltInDecorator::CompSubgroupId => naga::BuiltIn::SubgroupId,
    ShaderBuiltInDecorator::CompSubgroupSize => naga::BuiltIn::SubgroupSize,
    ShaderBuiltInDecorator::MeshPrimitiveTriangleIndex => naga::BuiltIn::TriangleIndices,
    ShaderBuiltInDecorator::MeshPrimitiveLineIndex => naga::BuiltIn::LineIndices,
    ShaderBuiltInDecorator::MeshPrimitivePointIndex => naga::BuiltIn::PointIndex,
    ShaderBuiltInDecorator::MeshPrimitiveCount => naga::BuiltIn::PrimitiveCount,
    ShaderBuiltInDecorator::MeshVertexCount => naga::BuiltIn::VertexCount,
    ShaderBuiltInDecorator::MeshVerticesOutput => naga::BuiltIn::Vertices,
    ShaderBuiltInDecorator::MeshPrimitiveOutput => naga::BuiltIn::Primitives,
  }
}

// workaround chrome bug
fn workaround_f32_max(f: f32) -> f32 {
  if f == f32::MAX { f.next_down() } else { f }
}

pub fn scalar_value_to_naga_literal(v: ScalarValue) -> naga::Literal {
  match v {
    ScalarValue::F32(v) => naga::Literal::F32(workaround_f32_max(v)),
    ScalarValue::U32(v) => naga::Literal::U32(v),
    ScalarValue::I32(v) => naga::Literal::I32(v),
    ScalarValue::Bool(v) => naga::Literal::Bool(v),
  }
}

pub fn map_binary_op(o: BinaryOperator) -> naga::BinaryOperator {
  match o {
    BinaryOperator::Add => naga::BinaryOperator::Add,
    BinaryOperator::Sub => naga::BinaryOperator::Subtract,
    BinaryOperator::Mul => naga::BinaryOperator::Multiply,
    BinaryOperator::Div => naga::BinaryOperator::Divide,
    BinaryOperator::Rem => naga::BinaryOperator::Modulo,
    BinaryOperator::Eq => naga::BinaryOperator::Equal,
    BinaryOperator::NotEq => naga::BinaryOperator::NotEqual,
    BinaryOperator::GreaterThan => naga::BinaryOperator::Greater,
    BinaryOperator::LessThan => naga::BinaryOperator::Less,
    BinaryOperator::GreaterEqualThan => naga::BinaryOperator::GreaterEqual,
    BinaryOperator::LessEqualThan => naga::BinaryOperator::LessEqual,
    BinaryOperator::LogicalOr => naga::BinaryOperator::LogicalOr,
    BinaryOperator::LogicalAnd => naga::BinaryOperator::LogicalAnd,
    BinaryOperator::BitAnd => naga::BinaryOperator::And,
    BinaryOperator::BitOr => naga::BinaryOperator::InclusiveOr,
    BinaryOperator::BitXor => naga::BinaryOperator::ExclusiveOr,
    BinaryOperator::ShiftLeft => naga::BinaryOperator::ShiftLeft,
    BinaryOperator::ShiftRight => naga::BinaryOperator::ShiftRight,
  }
}

pub fn map_unary_operator(o: UnaryOperator) -> naga::UnaryOperator {
  match o {
    UnaryOperator::LogicalNot => naga::UnaryOperator::LogicalNot,
    UnaryOperator::BitwiseNot => naga::UnaryOperator::BitwiseNot,
    UnaryOperator::Neg => naga::UnaryOperator::Negate,
  }
}

pub fn map_math_function(f: ShaderBuiltInFunction) -> naga::MathFunction {
  match f {
    ShaderBuiltInFunction::Select
    | ShaderBuiltInFunction::All
    | ShaderBuiltInFunction::Any
    | ShaderBuiltInFunction::IsNan
    | ShaderBuiltInFunction::IsInf
    | ShaderBuiltInFunction::ArrayLength => {
      unreachable!("these builtin functions produce dedicated expression form")
    }
    ShaderBuiltInFunction::Transpose => naga::MathFunction::Transpose,
    ShaderBuiltInFunction::Normalize => naga::MathFunction::Normalize,
    ShaderBuiltInFunction::Length => naga::MathFunction::Length,
    ShaderBuiltInFunction::Dot => naga::MathFunction::Dot,
    ShaderBuiltInFunction::Cross => naga::MathFunction::Cross,
    ShaderBuiltInFunction::SmoothStep => naga::MathFunction::SmoothStep,
    ShaderBuiltInFunction::Min => naga::MathFunction::Min,
    ShaderBuiltInFunction::Max => naga::MathFunction::Max,
    ShaderBuiltInFunction::Clamp => naga::MathFunction::Clamp,
    ShaderBuiltInFunction::Abs => naga::MathFunction::Abs,
    ShaderBuiltInFunction::Pow => naga::MathFunction::Pow,
    ShaderBuiltInFunction::Saturate => naga::MathFunction::Saturate,
    ShaderBuiltInFunction::Cos => naga::MathFunction::Cos,
    ShaderBuiltInFunction::Cosh => naga::MathFunction::Cosh,
    ShaderBuiltInFunction::Sin => naga::MathFunction::Sin,
    ShaderBuiltInFunction::Sinh => naga::MathFunction::Sinh,
    ShaderBuiltInFunction::Tan => naga::MathFunction::Tan,
    ShaderBuiltInFunction::Tanh => naga::MathFunction::Tanh,
    ShaderBuiltInFunction::Acos => naga::MathFunction::Acos,
    ShaderBuiltInFunction::Asin => naga::MathFunction::Asin,
    ShaderBuiltInFunction::Atan => naga::MathFunction::Atan,
    ShaderBuiltInFunction::Atan2 => naga::MathFunction::Atan2,
    ShaderBuiltInFunction::Asinh => naga::MathFunction::Asinh,
    ShaderBuiltInFunction::Acosh => naga::MathFunction::Acosh,
    ShaderBuiltInFunction::Atanh => naga::MathFunction::Atanh,
    ShaderBuiltInFunction::Radians => naga::MathFunction::Radians,
    ShaderBuiltInFunction::Degrees => naga::MathFunction::Degrees,
    ShaderBuiltInFunction::Ceil => naga::MathFunction::Ceil,
    ShaderBuiltInFunction::Floor => naga::MathFunction::Floor,
    ShaderBuiltInFunction::Round => naga::MathFunction::Round,
    ShaderBuiltInFunction::Fract => naga::MathFunction::Fract,
    ShaderBuiltInFunction::Trunc => naga::MathFunction::Trunc,
    ShaderBuiltInFunction::Modf => naga::MathFunction::Modf,
    ShaderBuiltInFunction::Frexp => naga::MathFunction::Frexp,
    ShaderBuiltInFunction::Ldexp => naga::MathFunction::Ldexp,
    ShaderBuiltInFunction::Exp => naga::MathFunction::Exp,
    ShaderBuiltInFunction::Exp2 => naga::MathFunction::Exp2,
    ShaderBuiltInFunction::Log => naga::MathFunction::Log,
    ShaderBuiltInFunction::Log2 => naga::MathFunction::Log2,
    ShaderBuiltInFunction::Outer => naga::MathFunction::Outer,
    ShaderBuiltInFunction::Distance => naga::MathFunction::Distance,
    ShaderBuiltInFunction::FaceForward => naga::MathFunction::FaceForward,
    ShaderBuiltInFunction::Reflect => naga::MathFunction::Reflect,
    ShaderBuiltInFunction::Refract => naga::MathFunction::Refract,
    ShaderBuiltInFunction::Sign => naga::MathFunction::Sign,
    ShaderBuiltInFunction::Fma => naga::MathFunction::Fma,
    ShaderBuiltInFunction::Mix => naga::MathFunction::Mix,
    ShaderBuiltInFunction::Step => naga::MathFunction::Step,
    ShaderBuiltInFunction::Sqrt => naga::MathFunction::Sqrt,
    ShaderBuiltInFunction::InverseSqrt => naga::MathFunction::InverseSqrt,
    ShaderBuiltInFunction::Inverse => naga::MathFunction::Inverse,
    ShaderBuiltInFunction::Determinant => naga::MathFunction::Determinant,
    ShaderBuiltInFunction::CountTrailingZeros => naga::MathFunction::CountTrailingZeros,
    ShaderBuiltInFunction::CountLeadingZeros => naga::MathFunction::CountLeadingZeros,
    ShaderBuiltInFunction::CountOneBits => naga::MathFunction::CountOneBits,
    ShaderBuiltInFunction::ReverseBits => naga::MathFunction::ReverseBits,
    ShaderBuiltInFunction::ExtractBits => naga::MathFunction::ExtractBits,
    ShaderBuiltInFunction::InsertBits => naga::MathFunction::InsertBits,
    ShaderBuiltInFunction::Pack4x8snorm => naga::MathFunction::Pack4x8snorm,
    ShaderBuiltInFunction::Pack4x8unorm => naga::MathFunction::Pack4x8unorm,
    ShaderBuiltInFunction::Pack2x16snorm => naga::MathFunction::Pack2x16snorm,
    ShaderBuiltInFunction::Pack2x16unorm => naga::MathFunction::Pack2x16unorm,
    ShaderBuiltInFunction::Pack2x16float => naga::MathFunction::Pack2x16float,
    ShaderBuiltInFunction::Unpack4x8snorm => naga::MathFunction::Unpack4x8snorm,
    ShaderBuiltInFunction::Unpack4x8unorm => naga::MathFunction::Unpack4x8unorm,
    ShaderBuiltInFunction::Unpack2x16snorm => naga::MathFunction::Unpack2x16snorm,
    ShaderBuiltInFunction::Unpack2x16unorm => naga::MathFunction::Unpack2x16unorm,
    ShaderBuiltInFunction::Unpack2x16float => naga::MathFunction::Unpack2x16float,
  }
}

pub fn map_atomic_function(
  function: AtomicFunction,
  compare: Option<naga::Handle<naga::Expression>>,
) -> naga::AtomicFunction {
  match function {
    AtomicFunction::Add => naga::AtomicFunction::Add,
    AtomicFunction::Subtract => naga::AtomicFunction::Subtract,
    AtomicFunction::And => naga::AtomicFunction::And,
    AtomicFunction::ExclusiveOr => naga::AtomicFunction::ExclusiveOr,
    AtomicFunction::InclusiveOr => naga::AtomicFunction::InclusiveOr,
    AtomicFunction::Min => naga::AtomicFunction::Min,
    AtomicFunction::Max => naga::AtomicFunction::Max,
    AtomicFunction::Exchange { .. } => naga::AtomicFunction::Exchange { compare },
  }
}

pub fn map_sample_level(
  level: SampleLevel,
  mut resolve: impl FnMut(ShaderNodeRawHandle) -> naga::Handle<naga::Expression>,
) -> naga::SampleLevel {
  match level {
    SampleLevel::Auto => naga::SampleLevel::Auto,
    SampleLevel::Zero => naga::SampleLevel::Zero,
    SampleLevel::Exact(handle) => naga::SampleLevel::Exact(resolve(handle)),
    SampleLevel::Bias(handle) => naga::SampleLevel::Bias(resolve(handle)),
    SampleLevel::Gradient { x, y } => naga::SampleLevel::Gradient {
      x: resolve(x),
      y: resolve(y),
    },
  }
}

pub fn map_texture_query(
  query: TextureQuery,
  level: Option<naga::Handle<naga::Expression>>,
) -> naga::ImageQuery {
  match query {
    TextureQuery::Size { .. } => naga::ImageQuery::Size { level },
    TextureQuery::NumLevels => naga::ImageQuery::NumLevels,
    TextureQuery::NumLayers => naga::ImageQuery::NumLayers,
    TextureQuery::NumSamples => naga::ImageQuery::NumSamples,
  }
}

pub fn map_gather_channel(channel: GatherChannel) -> naga::SwizzleComponent {
  match channel {
    GatherChannel::X => naga::SwizzleComponent::X,
    GatherChannel::Y => naga::SwizzleComponent::Y,
    GatherChannel::Z => naga::SwizzleComponent::Z,
    GatherChannel::W => naga::SwizzleComponent::W,
  }
}

pub fn map_derivative_axis(axis: DerivativeAxis) -> naga::DerivativeAxis {
  match axis {
    DerivativeAxis::X => naga::DerivativeAxis::X,
    DerivativeAxis::Y => naga::DerivativeAxis::Y,
    DerivativeAxis::Width => naga::DerivativeAxis::Width,
  }
}

pub fn map_derivative_control(ctrl: DerivativeControl) -> naga::DerivativeControl {
  match ctrl {
    DerivativeControl::Coarse => naga::DerivativeControl::Coarse,
    DerivativeControl::Fine => naga::DerivativeControl::Fine,
    DerivativeControl::None => naga::DerivativeControl::None,
  }
}

pub fn map_subgroup_operation(op: SubgroupOperation) -> naga::SubgroupOperation {
  match op {
    SubgroupOperation::All => naga::SubgroupOperation::All,
    SubgroupOperation::Any => naga::SubgroupOperation::Any,
    SubgroupOperation::Add => naga::SubgroupOperation::Add,
    SubgroupOperation::Mul => naga::SubgroupOperation::Mul,
    SubgroupOperation::Min => naga::SubgroupOperation::Min,
    SubgroupOperation::Max => naga::SubgroupOperation::Max,
    SubgroupOperation::And => naga::SubgroupOperation::And,
    SubgroupOperation::Or => naga::SubgroupOperation::Or,
    SubgroupOperation::Xor => naga::SubgroupOperation::Xor,
  }
}

pub fn map_collective_operation(op: SubgroupCollectiveOperation) -> naga::CollectiveOperation {
  match op {
    SubgroupCollectiveOperation::Reduce => naga::CollectiveOperation::Reduce,
    SubgroupCollectiveOperation::InclusiveScan => naga::CollectiveOperation::InclusiveScan,
    SubgroupCollectiveOperation::ExclusiveScan => naga::CollectiveOperation::ExclusiveScan,
  }
}

pub fn map_subgroup_gather_mode(
  mode: SubgroupGatherMode,
  mut resolve: impl FnMut(ShaderNodeRawHandle) -> naga::Handle<naga::Expression>,
) -> naga::GatherMode {
  match mode {
    SubgroupGatherMode::BroadcastFirst => naga::GatherMode::BroadcastFirst,
    SubgroupGatherMode::Broadcast(handle) => naga::GatherMode::Broadcast(resolve(handle)),
    SubgroupGatherMode::Shuffle(handle) => naga::GatherMode::Shuffle(resolve(handle)),
    SubgroupGatherMode::ShuffleDown(handle) => naga::GatherMode::ShuffleDown(resolve(handle)),
    SubgroupGatherMode::ShuffleUp(handle) => naga::GatherMode::ShuffleUp(resolve(handle)),
    SubgroupGatherMode::ShuffleXor(handle) => naga::GatherMode::ShuffleXor(resolve(handle)),
  }
}
