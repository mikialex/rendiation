/// wgpu tracing issue: https://github.com/gfx-rs/wgpu/issues/5555
/// corresponding spec: https://www.w3.org/TR/WGSL/#subgroup-builtin-functions
use crate::*;

// numeric scalar or numeric vector
impl<T> Node<T> {
  /// Returns the sum of self among all active invocations in the subgroup
  pub fn subgroup_add(&self) -> Self {
    todo!()
  }

  /// Returns the sum of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than the current invocation’s id.
  ///
  /// The value returned for the invocation with the lowest id among active invocations is 0
  pub fn subgroup_exclusive_add(&self) -> Self {
    todo!()
  }

  /// Returns the sum of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than or equal to the current invocation’s id.
  ///
  /// equivalent to `self.subgroup_exclusive_add() + self`.
  pub fn subgroup_inclusive_add(&self) -> Self {
    todo!()
  }

  /// Returns the product of self among all active invocations in the subgroup
  pub fn subgroup_mul(&self) -> Self {
    todo!()
  }

  /// Returns the product of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than the current invocation’s id.
  ///
  /// The value returned for the invocation with the lowest id among active invocations is 0
  pub fn subgroup_exclusive_mul(&self) -> Self {
    todo!()
  }

  /// Returns the product of self among all active invocations in the subgroup
  /// whose subgroup invocation IDs are less than or equal to the current invocation’s id.
  ///
  /// equivalent to `self.subgroup_exclusive_mul() * self`.
  pub fn subgroup_inclusive_mul(&self) -> Self {
    todo!()
  }

  /// Returns the value of e from the invocation whose subgroup invocation ID matches
  /// id in the subgroup to all active invocations in the subgroup.
  ///
  /// id must in the range [0, 128).
  pub fn subgroup_broadcast(&self, id: u32) -> Self {
    todo!()
  }
  /// Returns the value of e from the invocation that has the lowest subgroup invocation
  /// ID among active invocations in the subgroup to all active invocations in the subgroup.
  pub fn subgroup_broadcast_first(&self) -> Self {
    todo!()
  }
  /// Returns self from the invocation whose subgroup invocation ID matches id.
  /// id should inside the range [0, 128)
  pub fn subgroup_shuffle(&self, id: Node<u32>) -> Self {
    todo!()
  }
  /// Returns self from the invocation whose subgroup invocation ID
  /// matches subgroup_invocation_id - delta for the current invocation.
  ///
  /// delta should be in the range [0, 128), and uniform
  pub fn subgroup_shuffle_up(&self, delta: Node<u32>) -> Self {
    todo!()
  }

  /// Returns self from the invocation whose subgroup invocation ID
  /// matches subgroup_invocation_id + delta for the current invocation.
  ///
  /// delta should be in the range [0, 128), and uniform
  pub fn subgroup_shuffle_down(&self, delta: Node<u32>) -> Self {
    todo!()
  }

  /// Returns the maximum value of self among all active invocations in the subgroup.
  pub fn subgroup_max(&self) -> Self {
    todo!()
  }

  /// Returns the minimum value of self among all active invocations in the subgroup.
  pub fn subgroup_min(&self) -> Self {
    todo!()
  }
}

impl Node<bool> {
  /// Returns true if self is true for all active invocations in the subgroup.
  pub fn subgroup_all(&self) -> Self {
    todo!()
  }

  /// Returns true if self is true for any active invocations in the subgroup.
  pub fn subgroup_any(&self) -> Self {
    todo!()
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
  pub fn subgroup_ballot(&self) -> Node<u32> {
    todo!()
  }
}

// todo VecN<u32>, i32, VecN<i32>
impl Node<u32> {
  /// Returns the bitwise and (&) of self among all active invocations in the subgroup.
  pub fn subgroup_and(&self) -> Self {
    todo!()
  }
  /// Returns the bitwise or (|) of self among all active invocations in the subgroup.
  pub fn subgroup_or(&self) -> Self {
    todo!()
  }
  /// Returns the bitwise xor (^) of self among all active invocations in the subgroup.
  pub fn subgroup_xor(&self) -> Self {
    todo!()
  }
}

/// Returns true if the current invocation has the lowest subgroup invocation ID among
/// active invocations in the subgroup.
pub fn subgroup_elect() -> Node<bool> {
  todo!()
}

pub fn subgroup_invocation_id() -> Node<u32> {
  todo!()
}
