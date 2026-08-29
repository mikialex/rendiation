//! corresponding spec: https://www.w3.org/TR/WGSL/#subgroup-builtin-functions
use crate::*;

pub trait ShaderNumericScalar {}
impl ShaderNumericScalar for f32 {}
impl ShaderNumericScalar for u32 {}
impl ShaderNumericScalar for i32 {}

pub trait NumericScalarOrVector {}
impl<T> NumericScalarOrVector for T where T: ShaderNumericScalar {}
impl<T: ShaderNumericScalar> NumericScalarOrVector for Vec2<T> {}
impl<T: ShaderNumericScalar> NumericScalarOrVector for Vec3<T> {}
impl<T: ShaderNumericScalar> NumericScalarOrVector for Vec4<T> {}

impl<T: NumericScalarOrVector + PrimitiveShaderNodeType> Node<T> {
  /// Returns the sum of self among all active invocations in the subgroup
  pub fn subgroup_add(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Add,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the sum of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than the current invocation’s id.
  ///
  /// The value returned for the invocation with the lowest id among active invocations is 0
  pub fn subgroup_exclusive_add(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Add,
      SubgroupCollectiveOperation::ExclusiveScan,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the sum of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than or equal to the current invocation’s id.
  ///
  /// equivalent to `self.subgroup_exclusive_add() + self`.
  pub fn subgroup_inclusive_add(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Add,
      SubgroupCollectiveOperation::InclusiveScan,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the product of self among all active invocations in the subgroup
  pub fn subgroup_mul(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Mul,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the product of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than the current invocation’s id.
  ///
  /// The value returned for the invocation with the lowest id among active invocations is 0
  pub fn subgroup_exclusive_mul(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Mul,
      SubgroupCollectiveOperation::ExclusiveScan,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the product of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than or equal to the current invocation’s id.
  ///
  /// equivalent to `self.subgroup_exclusive_mul() * self`.
  pub fn subgroup_inclusive_mul(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Mul,
      SubgroupCollectiveOperation::InclusiveScan,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the value of e from the invocation whose subgroup invocation ID matches
  /// id in the subgroup to all active invocations in the subgroup.
  ///
  /// id must in the range [0, 128).
  pub fn subgroup_broadcast(&self, id: u32) -> Self {
    make_subgroup_gather_op(
      SubgroupGatherMode::Broadcast(val(id).handle()),
      self.handle(),
      T::primitive_ty(),
    )
  }
  /// Returns the value of e from the invocation that has the lowest subgroup invocation
  /// ID among active invocations in the subgroup to all active invocations in the subgroup.
  pub fn subgroup_broadcast_first(&self) -> Self {
    make_subgroup_gather_op(
      SubgroupGatherMode::BroadcastFirst,
      self.handle(),
      T::primitive_ty(),
    )
  }
  /// Returns self from the invocation whose subgroup invocation ID matches id.
  /// id should inside the range [0, 128)
  pub fn subgroup_shuffle(&self, id: impl Into<Node<u32>>) -> Self {
    make_subgroup_gather_op(
      SubgroupGatherMode::Shuffle(id.into().handle()),
      self.handle(),
      T::primitive_ty(),
    )
  }
  /// Returns self from the invocation whose subgroup invocation ID
  /// matches subgroup_invocation_id - delta for the current invocation.
  ///
  /// delta should be in the range [0, 128), and uniform
  pub fn subgroup_shuffle_up(&self, delta: impl Into<Node<u32>>) -> Self {
    make_subgroup_gather_op(
      SubgroupGatherMode::ShuffleUp(delta.into().handle()),
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns self from the invocation whose subgroup invocation ID
  /// matches subgroup_invocation_id + delta for the current invocation.
  ///
  /// delta should be in the range [0, 128), and uniform
  pub fn subgroup_shuffle_down(&self, delta: impl Into<Node<u32>>) -> Self {
    make_subgroup_gather_op(
      SubgroupGatherMode::ShuffleDown(delta.into().handle()),
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the maximum value of self among all active invocations in the subgroup.
  pub fn subgroup_max(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Max,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }

  /// Returns the minimum value of self among all active invocations in the subgroup.
  pub fn subgroup_min(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Min,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }
}

impl Node<bool> {
  /// Returns true if self is true for all active invocations in the subgroup.
  pub fn subgroup_all(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::All,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      bool::primitive_ty(),
    )
  }

  /// Returns true if self is true for any active invocations in the subgroup.
  pub fn subgroup_any(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Any,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      bool::primitive_ty(),
    )
  }

  /// Returns a bitmask of the active invocations in the subgroup for whom pred is true.
  ///
  /// - The x component of the return value contains invocations 0 through 31.
  /// - The y component of the return value contains invocations 32 through 63.
  /// - The z component of the return value contains invocations 64 through 95.
  /// - The w component of the return value contains invocations 96 through 127.
  ///
  /// Within each component, the IDs are in ascending order by bit position
  /// (e.g. ID 32 is at bit position 0 in the y component).
  pub fn subgroup_ballot(&self) -> Node<Vec4<u32>> {
    call_shader_api(|api| unsafe {
      api
        .make_expression(ShaderNodeExpr::SubgroupBallot {
          predicate: self.handle(),
        })
        .into_node()
    })
  }
}

pub trait ShaderIntScalar: ShaderNumericScalar {}
impl ShaderIntScalar for u32 {}
impl ShaderIntScalar for i32 {}

pub trait IntScalarOrVector {}

impl<T> IntScalarOrVector for T where T: ShaderIntScalar {}
impl<T: ShaderIntScalar> IntScalarOrVector for Vec2<T> {}
impl<T: ShaderIntScalar> IntScalarOrVector for Vec3<T> {}
impl<T: ShaderIntScalar> IntScalarOrVector for Vec4<T> {}

impl<T: IntScalarOrVector + PrimitiveShaderNodeType> Node<T> {
  /// Returns the bitwise and (&) of self among all active invocations in the subgroup.
  pub fn subgroup_and(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::And,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }
  /// Returns the bitwise or (|) of self among all active invocations in the subgroup.
  pub fn subgroup_or(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Or,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }
  /// Returns the bitwise xor (^) of self among all active invocations in the subgroup.
  pub fn subgroup_xor(&self) -> Self {
    make_subgroup_collective_op(
      SubgroupOperation::Xor,
      SubgroupCollectiveOperation::Reduce,
      self.handle(),
      T::primitive_ty(),
    )
  }
}

/// Returns true if the current invocation has the lowest subgroup invocation ID among
/// active invocations in the subgroup.
pub fn subgroup_elect() -> Node<bool> {
  // see https://github.com/gfx-rs/wgpu/issues/5555
  unimplemented!()
}

#[repr(u32)]
pub enum SubgroupOperation {
  All = 0,
  Any = 1,
  Add = 2,
  Mul = 3,
  Min = 4,
  Max = 5,
  And = 6,
  Or = 7,
  Xor = 8,
}

pub enum SubgroupGatherMode {
  BroadcastFirst,
  Broadcast(ShaderNodeRawHandle),
  Shuffle(ShaderNodeRawHandle),
  ShuffleDown(ShaderNodeRawHandle),
  ShuffleUp(ShaderNodeRawHandle),
  ShuffleXor(ShaderNodeRawHandle),
}

#[repr(u32)]
pub enum SubgroupCollectiveOperation {
  Reduce = 0,
  InclusiveScan = 1,
  ExclusiveScan = 2,
}

fn make_subgroup_collective_op<T>(
  operation: SubgroupOperation,
  collective_operation: SubgroupCollectiveOperation,
  argument: ShaderNodeRawHandle,
  ty: PrimitiveShaderValueType,
) -> Node<T> {
  call_shader_api(|api| unsafe {
    api
      .make_expression(ShaderNodeExpr::SubgroupCollectiveOperation {
        operation,
        collective_operation,
        argument,
        ty,
      })
      .into_node()
  })
}

fn make_subgroup_gather_op<T>(
  mode: SubgroupGatherMode,
  argument: ShaderNodeRawHandle,
  ty: PrimitiveShaderValueType,
) -> Node<T> {
  call_shader_api(|api| unsafe {
    api
      .make_expression(ShaderNodeExpr::SubgroupGather { mode, argument, ty })
      .into_node()
  })
}
