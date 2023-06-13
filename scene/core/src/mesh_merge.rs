use std::collections::HashSet;

use rendiation_renderable_mesh::{MeshGroup, MeshGroupsInfo, PrimitiveTopology};

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
  if could_merge_together(inputs) {
    return Err(MergeError::CannotMergeDifferentTypes);
  }

  look_ahead_split(inputs, make_splitter())
    .map(|groups| merge_assume_all_suitable_and_fit(groups, position_mapper, normal_mapper))
    .try_collect()
}

// todo  we should allow u16 merge to u32
fn make_splitter() -> impl FnMut(Option<&&AttributesMesh>) -> bool {
  let mut current_vertex_count: u32 = 0;
  move |next_mesh| {
    if let Some(next_mesh) = next_mesh {
      let next_vertex_count = next_mesh.get_position().count;
      if let Some((fmt, _)) = &next_mesh.indices {
        let max = match fmt {
          IndexFormat::Uint16 => u16::MAX as u32,
          IndexFormat::Uint32 => u32::MAX,
        };

        if max - current_vertex_count <= next_vertex_count as u32 {
          true
        } else {
          current_vertex_count += next_vertex_count as u32;
          false
        }
      } else {
        false
      }
    } else {
      true
    }
  }
}

fn look_ahead_split<T>(
  input: &[T],
  splitter: impl FnMut(Option<&T>) -> bool,
) -> impl Iterator<Item = &[T]> {
  LookAheadSplit { input, splitter }
}

struct LookAheadSplit<'a, T, F> {
  input: &'a [T],
  splitter: F,
}

impl<'a, T, F: FnMut(Option<&T>) -> bool> Iterator for LookAheadSplit<'a, T, F> {
  type Item = &'a [T];

  fn next(&mut self) -> Option<Self::Item> {
    let mut id = 0;
    for idx in 0..self.input.len() {
      if !(self.splitter)(self.input.get(idx + 1)) {
        id += 1;
      } else {
        break;
      }
    }

    let ret = Some(&self.input[..id]);
    self.input = &self.input[id..];
    ret
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeMeshMergeKey {
  pub attributes: HashSet<AttributeSemantic>, // todo use on stack hash set?,
  pub indices: Option<IndexFormat>,
  pub mode: PrimitiveTopology,
}

pub fn compute_merge_key(att: &&AttributesMesh) -> AttributeMeshMergeKey {
  AttributeMeshMergeKey {
    attributes: att.attributes.iter().map(|(k, _)| *k).collect(),
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

  let vertex_prefix_sum = prefix_scan(inputs.iter().map(|att| att.get_position().count));

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

      let merged = match format {
        IndexFormat::Uint16 => merge_attribute_accessor::<u16>(&to_merge, index_reducer_16),
        IndexFormat::Uint32 => merge_attribute_accessor::<u32>(&to_merge, index_reducer_32),
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
/// todo move to math lib
trait Monoid {
  /// Combines two monoids. This operation must be associative.
  fn combine(&self, other: &Self) -> Self;
  fn identity() -> Self;
}

impl Monoid for usize {
  fn combine(&self, other: &Self) -> Self {
    self + other
  }

  fn identity() -> Self {
    0
  }
}

// todo return iter
fn prefix_scan<T: Monoid>(input: impl Iterator<Item = T>) -> Vec<T> {
  let result = Vec::new();
  let id = T::identity();

  // todo improve
  input.fold(result, |mut result, current| {
    let last = result.last().unwrap_or(&id);
    result.push(last.combine(&current));
    result
  })
}
