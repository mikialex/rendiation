use std::sync::Arc;

use rayon::prelude::*;
use rendiation_geometry::Box3;
use rendiation_mesh_core::{AttributeSemantic, MeshPrimitiveTopology};
use rendiation_mesh_simplification::*;

use crate::*;

pub type AttributesLODMeshMetadataChange =
  Arc<LinearBatchChanges<RawEntityHandle, ExternalRefPtr<Vec<LODLevelInfo>>>>;

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, PartialEq, Copy, ShaderStruct, Default)]
pub struct LODLevelInfo {
  /// Relative to the mesh itself's all lod level's indices, not the global indices pool.
  /// The offset unit is u32 slots(4 bytes), same as [AttributeMeshMeta::index_offset],
  /// so the shader can combine them directly in both native and midc downgrade mode.
  /// The per level alignment is done by the padding in [build_merged_lod_mesh].
  pub index_offset: u32,
  /// the index element count of this level, the element is u16 or u32 depends on the mesh's index format
  pub count: u32,
  /// The absolute max distance between the mesh and simplified mesh in mesh's local space
  pub error: f32,
}

pub struct AttributeMeshLODConvertResult {
  pub processed_meshes: UseResult<AttributesMeshDataChangeInput>,
  pub lod_metadata: UseResult<AttributesLODMeshMetadataChange>,
}

pub fn process_attribute_mesh_lod(
  cx: &mut impl QueryHookCxLike,
  mesh_changes: UseResult<AttributesMeshDataChangeInput>,
  lod_config: &AttributeLODConfig,
) -> AttributeMeshLODConvertResult {
  let spawner = cx.spawner();
  let lod_config = lod_config.clone();
  let (converted, converted_) = mesh_changes
    .map_spawn_stage_in_thread_data_changes(cx, move |meshes_changes| {
      let meshes_changes = meshes_changes.materialize();

      let spawner = spawner.unwrap();

      // the simplification of each mesh is independent, run them in parallel
      // in the project's own rayon pool instead of the global one
      let items: Vec<_> = meshes_changes.iter_update_or_insert().collect();
      let (processed_meshes, level_infos): (Vec<_>, Vec<_>) = spawner.install(|| {
        items
          .into_par_iter()
          .map(|(id, new_mesh)| {
            if let Some(new_mesh_) = new_mesh.if_loaded_ref() {
              let processed = process_lod_attribute_mesh(new_mesh_, &lod_config);
              (
                (id, UriLoadResult::LivingOrLoaded(processed.content)),
                Some((id, processed.lod_levels)),
              )
            } else {
              // level's output is skipped for this case
              ((id, new_mesh.clone()), None)
            }
          })
          .unzip()
      });
      let level_infos: Vec<_> = level_infos.into_iter().flatten().collect();

      let processed_meshes = Arc::new(LinearBatchChanges {
        removed: meshes_changes.removed.clone(),
        update_or_insert: processed_meshes,
      });

      let level_infos = Arc::new(LinearBatchChanges {
        removed: meshes_changes.removed.clone(),
        update_or_insert: level_infos,
      });

      (processed_meshes, level_infos)
    })
    .fork();

  let processed_meshes = converted.map(|v| v.0);
  let lod_metadata = converted_.map(|v| v.1);

  AttributeMeshLODConvertResult {
    processed_meshes,
    lod_metadata,
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[derive(Serialize, Deserialize)]
pub enum LODConversionMode {
  /// disable the lod conversion (as well as the lod effect)
  Disabled,
  /// each level's index count is the half of the previous level, edge collapse first, sloppy as fallback
  HalfCount,
  /// each level's error is doubled, edge collapse is dominated by the error limit
  ErrorDoubling,
}

#[derive(Copy, Clone, Debug)]
#[derive(Serialize, Deserialize)]
pub struct ErrorDoublingConfig {
  /// base error factor, relative to the mesh's bounding box extent
  pub base_error_factor: f32,
  /// max error factor
  pub max_error_factor: f32,
}

impl Default for ErrorDoublingConfig {
  fn default() -> Self {
    Self {
      base_error_factor: 1e-3,
      max_error_factor: 0.1,
    }
  }
}

// todo return error type and warn invalid input
//
/// if the mesh is not able to do lod for some reason, just output the origin index as the only level
fn process_lod_attribute_mesh(
  input_mesh: &AttributesMeshWithVertexRelationInfo,
  lod_config: &AttributeLODConfig,
) -> AttributeLODMeshData {
  let Some(indices) = &input_mesh.indices else {
    return only_origin_level(input_mesh);
  };
  if input_mesh.mode != MeshPrimitiveTopology::TriangleList {
    return only_origin_level(input_mesh);
  }

  let indices_bytes = indices.byte_view();
  let byte_per_item = indices_bytes.len() / indices.count;
  if byte_per_item != 2 && byte_per_item != 4 {
    return only_origin_level(input_mesh);
  }

  let indices_u32: Vec<u32> = if byte_per_item == 2 {
    let Ok(indices_u16) = bytemuck::try_cast_slice::<u8, u16>(indices_bytes) else {
      return only_origin_level(input_mesh);
    };
    indices_u16.iter().map(|v| *v as u32).collect()
  } else {
    let Ok(indices_u32) = bytemuck::try_cast_slice::<u8, u32>(indices_bytes) else {
      return only_origin_level(input_mesh);
    };
    indices_u32.to_vec()
  };

  let triangle_count = indices.count / 3;
  if triangle_count < lod_config.min_lod_triangle_count {
    return only_origin_level(input_mesh);
  }

  let Some(positions_vertex) = input_mesh
    .vertices
    .iter()
    .find(|v| v.semantic == AttributeSemantic::Positions)
  else {
    return only_origin_level(input_mesh);
  };
  let Ok(positions) = bytemuck::try_cast_slice::<u8, Vec3<f32>>(positions_vertex.data.byte_view())
  else {
    return only_origin_level(input_mesh);
  };
  let vertex_count = positions.len();
  if vertex_count == 0 {
    return only_origin_level(input_mesh);
  }

  // the simplification references the origin vertex buffer, so the indices must be in range
  if indices_u32
    .iter()
    .copied()
    .max()
    .map(|max| max >= vertex_count as u32)
    .unwrap_or(true)
  {
    return only_origin_level(input_mesh);
  }

  let bbox: Box3 = positions.iter().copied().collect();
  let box_size = bbox.size();
  let extent = box_size.x.max(box_size.y).max(box_size.z);
  // the simplify rescales the mesh into unit cube, zero extent will break the scale
  if extent <= 1e-6 {
    return only_origin_level(input_mesh);
  }

  let levels = match lod_config.lod_conversion_mode {
    LODConversionMode::HalfCount => {
      simplify_half_count(lod_config, &indices_u32, positions, extent)
    }
    LODConversionMode::ErrorDoubling => {
      simplify_error_doubling(lod_config, &indices_u32, positions, extent)
    }
    LODConversionMode::Disabled => return only_origin_level(input_mesh),
  };

  build_merged_lod_mesh(input_mesh, &indices_u32, byte_per_item == 2, &levels)
}

struct SimplifiedLevel {
  indices: Vec<u32>,
  error: f32,
}

fn simplify_half_count(
  lod_config: &AttributeLODConfig,
  indices: &[u32],
  positions: &[Vec3<f32>],
  extent: f32,
) -> Vec<SimplifiedLevel> {
  let mut levels = Vec::new();
  let mut prev_count = indices.len();
  let mut prev_error = 0.;
  let mut dst = vec![0u32; indices.len()];
  let mut fallback_dst = vec![0u32; indices.len()];

  loop {
    let target = prev_count / 2;
    if target < lod_config.min_lod_triangle_count * 3 {
      break;
    }

    let result = simplify_by_edge_collapse(
      &mut dst,
      indices,
      positions,
      None,
      EdgeCollapseConfig {
        target_index_count: target,
        // not limited by the error, only limited by the count
        target_error: f32::MAX,
        use_absolute_error: true,
        lock_border: true,
      },
    );

    let (result_count, result_error, source) = if result.result_count <= target {
      (result.result_count, result.result_error, &dst[..])
    } else {
      // the topology is too complex to reach the half count, use sloppy to guarantee the count
      // note sloppy's target_error must not exceed the extent, otherwise the internal grid
      // size calculation will break
      let fallback = simplify_sloppy(
        &mut fallback_dst,
        indices,
        positions,
        None,
        target as u32,
        extent,
        true,
      );
      (
        fallback.result_count,
        fallback.result_error,
        &fallback_dst[..],
      )
    };

    if result_count >= prev_count || result_count < lod_config.min_lod_triangle_count * 3 {
      break;
    }

    // the sloppy's error metric is not comparable with the edge collapse's, force the
    // error to be monotonic increasing, the gpu selection relies on this property
    let error = result_error.max(prev_error);

    levels.push(SimplifiedLevel {
      indices: source[0..result_count].to_vec(),
      error,
    });
    prev_count = result_count;
    prev_error = error;
  }

  levels
}

fn simplify_error_doubling(
  lod_config: &AttributeLODConfig,
  indices: &[u32],
  positions: &[Vec3<f32>],
  extent: f32,
) -> Vec<SimplifiedLevel> {
  let mut levels = Vec::new();
  let mut prev_count = indices.len();
  let mut prev_error = 0.;
  let mut dst = vec![0u32; indices.len()];

  let mut target_error = extent * lod_config.error_double_mode_config.base_error_factor;
  let max_error = extent * lod_config.error_double_mode_config.max_error_factor;

  loop {
    if target_error > max_error || prev_count < lod_config.min_lod_triangle_count * 3 {
      break;
    }

    // todo, we should optimize this to do multi level at once?
    let result = simplify_by_edge_collapse(
      &mut dst,
      indices,
      positions,
      None,
      EdgeCollapseConfig {
        // the error limit dominates the simplification
        target_index_count: lod_config.min_lod_triangle_count * 3,
        target_error,
        use_absolute_error: true,
        lock_border: true,
      },
    );

    if result.result_count >= prev_count
      || result.result_count < lod_config.min_lod_triangle_count * 3
    {
      break;
    }

    // force the error to be monotonic increasing, the gpu selection relies on this property
    let error = result.result_error.max(prev_error);

    levels.push(SimplifiedLevel {
      indices: dst[0..result.result_count].to_vec(),
      error,
    });
    prev_count = result.result_count;
    prev_error = error;
    target_error *= 2.;
  }

  levels
}

/// merge all levels into one index buffer, the origin index is the first level as:
///
/// [origin_index, coarser_level_index, coarser_level_index, ...]
///
/// It is required because the mesh data may directly used in ctx that assume it's not lod-ed at all(for example rtx mesh data access)
fn build_merged_lod_mesh(
  input_mesh: &AttributesMeshWithVertexRelationInfo,
  origin_indices: &[u32],
  is_origin_mesh_u16: bool,
  simplified_levels: &[SimplifiedLevel],
) -> AttributeLODMeshData {
  let mut levels = Vec::with_capacity(simplified_levels.len() + 1);
  levels.push(LODLevelInfo {
    index_offset: 0,
    count: origin_indices.len() as u32,
    error: 0.,
    ..Default::default()
  });

  // we do not do u32 to u16 convert even the data fits, because the draw dispatcher
  // read origin mesh's index type to emit draw, here we must match.
  let content = if is_origin_mesh_u16 {
    let mut merged = Vec::<u16>::with_capacity(origin_indices.len() + simplified_levels.len() * 2);
    merged.extend(origin_indices.iter().map(|v| *v as u16));
    // pad the origin level's tail to an even element count as well, otherwise when the
    // origin element count is odd the first level's offset would fall on a u32 slot that
    // crosses the origin/level1 boundary, and the device base index can not express it
    if merged.len() % 2 == 1 {
      merged.push(0);
    }
    for level in simplified_levels {
      // the level's offset must be aligned to the u32 slot, otherwise in the midc downgrade
      // mode the u16 index pool is read as u32 slots on device and the base index can not
      // express the boundary of a level, so we pad the tail of every level to an even element
      // count here. the padded element is never drawn because the draw count is the real
      // element count, and the upload path pads the whole buffer tail to the 4-byte boundary
      // automatically, so this is the only alignment we need
      let offset_slots = merged.len() / 2;
      levels.push(LODLevelInfo {
        index_offset: offset_slots as u32,
        count: level.indices.len() as u32,
        error: level.error,
        ..Default::default()
      });
      merged.extend(level.indices.iter().map(|v| *v as u16));
      if merged.len() % 2 == 1 {
        merged.push(0);
      }
    }
    let bytes: Vec<u8> = bytemuck::cast_slice(&merged).to_vec();
    build_content(input_mesh, bytes, merged.len())
  } else {
    let mut merged = Vec::<u32>::with_capacity(origin_indices.len() + simplified_levels.len() * 2);
    merged.extend_from_slice(origin_indices);
    for level in simplified_levels {
      // u32 elements are naturally aligned to the u32 slot, no padding needed
      let offset_slots = merged.len();
      levels.push(LODLevelInfo {
        index_offset: offset_slots as u32,
        count: level.indices.len() as u32,
        error: level.error,
        ..Default::default()
      });
      merged.extend_from_slice(&level.indices);
    }
    let bytes: Vec<u8> = bytemuck::cast_slice(&merged).to_vec();
    build_content(input_mesh, bytes, merged.len())
  };

  AttributeLODMeshData {
    lod_levels: ExternalRefPtr::new(levels),
    content,
  }
}

fn build_content(
  input_mesh: &AttributesMeshWithVertexRelationInfo,
  data: Vec<u8>,
  element_count: usize,
) -> AttributesMeshWithVertexRelationInfo {
  AttributesMeshWithVertexRelationInfo {
    mode: input_mesh.mode,
    indices: Some(AttributeLivingData {
      data: Arc::new(data),
      range: None,
      count: element_count,
    }),
    vertices: input_mesh.vertices.clone(),
  }
}

/// the converted mesh's indices must contains origin index as the first level as:
///
/// [origin_index, coarser_level_index, coarser_level_index, ...]
///
/// It is required because the mesh data may directly used in ctx that assume it's not lod-ed at all(for example rtx mesh data access)
#[derive(Clone)]
pub struct AttributeLODMeshData {
  pub lod_levels: ExternalRefPtr<Vec<LODLevelInfo>>,
  /// draw mode must be triangle list
  ///
  /// must have indices
  pub content: AttributesMeshWithVertexRelationInfo,
}

fn only_origin_level(input_mesh: &AttributesMeshWithVertexRelationInfo) -> AttributeLODMeshData {
  let origin_count = input_mesh.indices.as_ref().map(|i| i.count).unwrap_or(0);
  AttributeLODMeshData {
    lod_levels: ExternalRefPtr::new(vec![LODLevelInfo {
      index_offset: 0,
      count: origin_count as u32,
      error: 0.,
      ..Default::default()
    }]),
    content: input_mesh.clone(),
  }
}
