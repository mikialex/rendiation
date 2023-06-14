use rendiation_renderable_mesh::{MeshGroup, MeshGroupsInfo, PrimitiveTopology};
use smallvec::SmallVec;

use crate::*;

#[derive(Debug)]
pub enum MergeError {
  CannotMergeDifferentTypes,
  UnsupportedAttributeType,
  AttributeDataAccessFailed,
}

pub fn merge(
  inputs: &[&AttributesMesh],
  position_mapper: impl Fn(usize, &Vec3<f32>) -> Vec3<f32> + Copy,
  normal_mapper: impl Fn(usize, &Vec3<f32>) -> Vec3<f32> + Copy,
) -> Result<Vec<AttributesMesh>, MergeError> {
  // check if inputs could merge together
  if !could_merge_together(inputs) {
    return Err(MergeError::CannotMergeDifferentTypes);
  }

  look_ahead_split(inputs, make_splitter())
    .map(|groups| merge_assume_all_suitable_and_fit(groups, position_mapper, normal_mapper))
    .try_collect()
}

// we are not considering the u16 merge into u32, because the u16 is big enough to achieve our goal
fn make_splitter() -> impl FnMut(&&AttributesMesh) -> bool {
  let mut current_vertex_count: u32 = 0;
  move |next_mesh| {
    let next_vertex_count = next_mesh.get_position().count;
    if let Some((fmt, _)) = &next_mesh.indices {
      let max = match fmt {
        AttributeIndexFormat::Uint16 => u16::MAX as u32,
        AttributeIndexFormat::Uint32 => u32::MAX,
      };

      if max - current_vertex_count <= next_vertex_count as u32 {
        current_vertex_count = 0;
        true
      } else {
        current_vertex_count += next_vertex_count as u32;
        false
      }
    } else {
      false
    }
  }
}

fn look_ahead_split<T>(
  input: &[T],
  splitter: impl FnMut(&T) -> bool,
) -> impl Iterator<Item = &[T]> {
  LookAheadSplit { input, splitter }
}

struct LookAheadSplit<'a, T, F> {
  input: &'a [T],
  splitter: F,
}

impl<'a, T, F: FnMut(&T) -> bool> Iterator for LookAheadSplit<'a, T, F> {
  type Item = &'a [T];

  fn next(&mut self) -> Option<Self::Item> {
    let idx = if let Some(id) = self.input.iter().position(|v| (self.splitter)(v)) {
      assert!(id >= 1);
      id - 1
    } else {
      0
    };

    let ret = &self.input[..idx];
    self.input = &self.input[idx..];
    ret.is_empty().then_some(ret)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeMeshMergeKey {
  pub attributes: SmallVec<[AttributeSemantic; 3]>,
  pub indices: Option<AttributeIndexFormat>,
  pub mode: PrimitiveTopology,
}

pub fn compute_merge_key(att: &&AttributesMesh) -> AttributeMeshMergeKey {
  let mut attributes: SmallVec<[AttributeSemantic; 3]> =
    att.attributes.iter().map(|(k, _)| *k).collect();
  attributes.sort();

  AttributeMeshMergeKey {
    attributes,
    indices: att.indices.as_ref().map(|(f, _)| *f),
    mode: att.mode,
  }
}

pub fn could_merge_together(inputs: &[&AttributesMesh]) -> bool {
  if let Some(first) = inputs.first() {
    let first_key = compute_merge_key(first);
    inputs
      .iter()
      .map(compute_merge_key)
      .all(|k| k.eq(&first_key))
  } else {
    false
  }
}

pub fn merge_attribute_accessor<T: bytemuck::Pod>(
  inputs: &[&AttributeAccessor],
  mut mapper: impl FnMut(usize, &T) -> T,
) -> Option<AttributeAccessor> {
  // todo stride support
  let first = inputs[0];

  let count = inputs.iter().map(|v| v.count).sum();
  let byte_count = std::mem::size_of::<T>() * count;

  let mut buffer = Vec::with_capacity(byte_count);
  for (idx, acc) in inputs.iter().enumerate() {
    acc.visit_slice::<T, _>(|s| {
      s.iter().for_each(|v| {
        buffer.extend(bytemuck::bytes_of(&mapper(idx, v)));
      })
    })?;
  }

  let buffer = GeometryBufferInner { buffer };
  let buffer = buffer.into_ref();
  let view = UnTypedBufferView {
    buffer,
    range: Default::default(),
  };
  AttributeAccessor {
    view,
    byte_offset: 0,
    count,
    item_size: first.item_size,
  }
  .into()
}

fn merge_assume_all_suitable_and_fit(
  inputs: &[&AttributesMesh],
  position_mapper: impl Fn(usize, &Vec3<f32>) -> Vec3<f32> + Copy,
  normal_mapper: impl Fn(usize, &Vec3<f32>) -> Vec3<f32> + Copy,
) -> Result<AttributesMesh, MergeError> {
  let first = inputs[0];

  let merged_attributes = first
    .attributes
    .iter()
    .map(|(key, _)| {
      let to_merge = inputs
        .iter()
        .map(|att| att.get_attribute(*key))
        .try_collect::<Vec<_>>()
        .ok_or(MergeError::CannotMergeDifferentTypes)?;

      use AttributeSemantic::*;
      let att = match key {
        Positions => merge_attribute_accessor::<Vec3<f32>>(&to_merge, position_mapper),
        Normals => merge_attribute_accessor::<Vec3<f32>>(&to_merge, normal_mapper),
        TexCoords(_) => merge_attribute_accessor::<Vec2<f32>>(&to_merge, |_, v| *v),
        _ => return Err(MergeError::UnsupportedAttributeType),
      }
      .ok_or(MergeError::AttributeDataAccessFailed)?;
      Ok((*key, att))
    })
    .try_collect::<Vec<_>>()?;

  let vertex_counts = inputs.iter().map(|att| att.get_position().count);
  let vertex_prefix_sum: Vec<_> = prefix_scan::<UsizeSum>(vertex_counts).collect();

  let merged_indices = first
    .indices
    .as_ref()
    .map(|(format, _)| {
      let to_merge = inputs
        .iter()
        .map(|att| att.indices.as_ref().map(|v| &v.1))
        .try_collect::<Vec<_>>()
        .ok_or(MergeError::CannotMergeDifferentTypes)?;

      let index_reducer_16 = |group_id, i: &u16| vertex_prefix_sum[group_id] as u16 + *i;
      let index_reducer_32 = |group_id, i: &u32| vertex_prefix_sum[group_id] as u32 + *i;

      use AttributeIndexFormat::*;
      let merged = match format {
        Uint16 => merge_attribute_accessor::<u16>(&to_merge, index_reducer_16),
        Uint32 => merge_attribute_accessor::<u32>(&to_merge, index_reducer_32),
      }
      .ok_or(MergeError::AttributeDataAccessFailed)?;
      Ok((*format, merged))
    })
    .transpose()?;

  let new_groups = vertex_prefix_sum
    .iter()
    .zip(inputs.iter().map(|g| &g.groups))
    .flat_map(|(&previous_summed, group)| {
      group.groups.iter().map(move |g| MeshGroup {
        start: g.start + previous_summed,
        count: g.count,
      })
    });

  let merged_groups = MeshGroupsInfo {
    groups: new_groups.collect(),
  };

  Ok(AttributesMesh {
    attributes: merged_attributes,
    indices: merged_indices,
    mode: first.mode,
    groups: merged_groups,
  })
}

/// https://en.wikipedia.org/wiki/Monoid
trait MonoidBehavior {
  type Value;
  /// Combines two monoids. This operation must be associative.
  fn combine(a: &Self::Value, b: &Self::Value) -> Self::Value;
  fn identity() -> Self::Value;
}

struct UsizeSum;

impl MonoidBehavior for UsizeSum {
  type Value = usize;
  fn combine(a: &Self::Value, b: &Self::Value) -> Self::Value {
    a + b
  }

  fn identity() -> Self::Value {
    0
  }
}

fn prefix_scan<T>(input: impl Iterator<Item = T::Value>) -> impl Iterator<Item = T::Value>
where
  T: MonoidBehavior,
  T::Value: Copy,
{
  input.scan(T::identity(), |summed, next| {
    *summed = T::combine(summed, &next);
    (*summed).into()
  })
}
