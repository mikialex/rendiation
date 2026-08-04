use crate::*;

/// the screen space error threshold to switch to a coarser level, in pixels
const LOD_ERROR_THRESHOLD_PIXELS: f32 = 2.;

#[derive(Clone)]
pub(super) struct AttributeLODMeshIndirectDrawCreator {
  pub(super) internal: AttributeMeshIndirectDrawCreator,
  pub(super) level_meta: AbstractReadonlyStorageBuffer<[Vec2<u32>]>,
  pub(super) lod_levels: AbstractReadonlyStorageBuffer<[LODLevelInfo]>,
  // used to scale the error metrics
  pub(super) sm_node_info: Box<dyn IndirectNodeInfoSceneModelAccess>,
  pub(super) sm_world_aabb_info: Box<dyn DrawUnitWorldBoundingProvider>,
  pub(super) lod_camera_info: LODCameraInfo,
}

#[derive(Clone)]
pub struct LODCameraInfo {
  pub camera: UniformBufferDataView<CameraGPUTransform>,
  // (width, height, padding, padding)
  pub view_resolution: UniformBufferDataView<Vec4<u32>>,
}

impl IndexedDrawCommandBuilder for AttributeLODMeshIndirectDrawCreator {
  fn draw_command_host_access(&self, _id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    // not supported yet.
    None
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation> {
    Box::new(AttributeMeshLODIndirectDrawCreatorInvocation {
      metadata: cx.bind_by(&self.internal.metadata),
      level_meta: cx.bind_by(&self.level_meta),
      sm_to_mesh_device: cx.bind_by(&self.internal.sm_to_mesh_device),
      lod_levels: cx.bind_by(&self.lod_levels),
      camera: cx.bind_by(&self.lod_camera_info.camera),
      view_resolution: cx.bind_by(&self.lod_camera_info.view_resolution),
      sm_node_info: self.sm_node_info.build(&mut cx.bindgroups),
      sm_world_aabb_info: self
        .sm_world_aabb_info
        .create_invocation(&mut cx.bindgroups),
      used_in_midc_downgrade: self.internal.used_in_midc_downgrade,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.internal.metadata);
    builder.bind(&self.level_meta);
    builder.bind(&self.internal.sm_to_mesh_device);
    builder.bind(&self.lod_levels);
    builder.bind(&self.lod_camera_info.camera);
    builder.bind(&self.lod_camera_info.view_resolution);
    self.sm_node_info.bind(builder);
    self.sm_world_aabb_info.bind(builder);
  }
}

impl ShaderHashProvider for AttributeLODMeshIndirectDrawCreator {
  shader_hash_type_id! {}
}

pub struct AttributeMeshLODIndirectDrawCreatorInvocation {
  metadata: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  level_meta: ShaderReadonlyPtrOf<[Vec2<u32>]>,
  lod_levels: ShaderReadonlyPtrOf<[LODLevelInfo]>,
  sm_to_mesh_device: ShaderReadonlyPtrOf<[u32]>,
  camera: ShaderReadonlyPtrOf<CameraGPUTransform>,
  view_resolution: ShaderReadonlyPtrOf<Vec4<u32>>,
  sm_node_info: Box<dyn IndirectNodeInfoSceneModelAccessInvocation>,
  sm_world_aabb_info: Box<dyn DrawUnitWorldBoundingInvocationProvider>,
  used_in_midc_downgrade: bool,
}

impl IndexedDrawCommandBuilderInvocation for AttributeMeshLODIndirectDrawCreatorInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    let meta = self.metadata.index(mesh_handle);

    // the fallback is the origin level draw, same as the mesh without any lod
    let is_u16 = meta.is_u16_indices().load().into_bool();
    let is_u16_padded = meta.is_u16_indices_padded().load().into_bool();

    // u16 indices are packed two per u32, an odd count leaves the last u32 holding only
    // one index plus 2 padding bytes, so the real index count is 2 * count - 1
    let meta_count = meta.count().load();
    let fallback_count = is_u16.select(meta_count * val(2), meta_count);
    let fallback_count = is_u16_padded.select(fallback_count - val(1), fallback_count);

    // the projected screen space error of a level:
    //   projected = world_error * viewport_height * focal_y / (2 * distance)   [perspective]
    //   projected = world_error * viewport_height * focal_y / 2                [orthographic]
    // the distance is the conservative closest distance from the camera to the world aabb,
    // so the projected error is never underestimated
    let camera_projection = self.camera.projection().load();
    let viewport_height: Node<f32> = self.view_resolution.load().y().into_f32();
    // the focal_y of a standard perspective matrix is 1/tan(fov/2), the orthographic
    // matrix is 2/frustum_height, both can be used as the pixel scale with the formula above
    let focal_y = camera_projection.y().y();

    // the standard perspective matrix has -1 in the w component of the z column,
    // while the orthographic matrix has 0, so it can be used to distinguish them,
    // the projected size of an orthographic camera does not depend on the distance
    let is_perspective = camera_projection.z().w().not_equals(val(0.));
    let distance_scale = is_perspective.select_branched(
      || {
        let camera_world_position = self.camera.world_position().load();
        let camera_pos = hpt_uniform_to_hpt(camera_world_position).expand();
        let bbox = self.sm_world_aabb_info.get_world_bounding(draw_id);
        let bbox_min = bbox.min.expand();
        let bbox_max = bbox.max.expand();
        // the lod selection is not sensitive to the distance precision, f32 is enough here
        let closest = camera_pos.f1.clamp(bbox_min.f1, bbox_max.f1);
        let distance = (camera_pos.f1 - closest).length().max(val(1e-6));
        val(1.) / distance
      },
      || val(1.),
    );
    let pixel_per_unit = viewport_height * focal_y / val(2.) * distance_scale;

    // scale the local space error to world space, use the max axis scale to stay conservative
    let node = self.sm_node_info.get_node_info_value(draw_id).expand();
    let mat = node.world_matrix_none_translation;
    let world_scale = mat
      .x()
      .xyz()
      .length()
      .max(mat.y().xyz().length())
      .max(mat.z().xyz().length());

    // the level selection: iterate from the coarsest level to the finest,
    // pick the first one whose projected error is under the threshold,
    // the level 0 is always the fallback
    let level_range = self.level_meta.index(mesh_handle).load();
    let level_start = level_range.x();
    let level_count = level_range.y();

    // the allocation is failed if the offset is the fail marker
    let has_lod = level_count
      .greater_than(val(1))
      .and(level_start.not_equals(val(DEVICE_RANGE_ALLOCATE_FAIL_MARKER)));

    let selected_offset = val(0u32).make_local_var();
    let selected_count = fallback_count.make_local_var();

    if_by(has_lod, || {
      let level_index = (level_count - val(1)).make_local_var();
      loop_by(|cx| {
        let info = self
          .lod_levels
          .index(level_start + level_index.load())
          .load()
          .expand();
        let projected_error = info.error * world_scale * pixel_per_unit;
        let should_select = projected_error
          .less_equal_than(val(LOD_ERROR_THRESHOLD_PIXELS))
          .or(level_index.load().equals(val(0)));
        if_by(should_select, || {
          selected_offset.store(info.index_offset);
          selected_count.store(info.count);
          cx.do_break();
        });
        level_index.store(level_index.load() - val(1));
      });
    });

    // in midc downgrade mode the pool is read on device with u32 indices, in native midc
    // draw the index buffer is bound as u16, so the first index is in u16 units, so it should be multiplied by 2
    // note the level offsets are aligned to u32 slots on the cpu side, so both modes can
    // combine the meta offset and the level offset directly
    let base_index = meta.index_offset().load() + selected_offset.load();
    let base_index = if self.used_in_midc_downgrade {
      base_index
    } else {
      is_u16.select(base_index * val(2), base_index)
    };

    ENode::<DrawIndexedIndirectArgsStorage> {
      vertex_count: selected_count.load(),
      instance_count: val(1),
      base_index,
      vertex_offset: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
