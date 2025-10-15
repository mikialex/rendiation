use bytemuck::cast_slice;
use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

pub struct SceneRayQuery {
  pub world_ray: Ray3<f64>,
  pub camera_view_size_in_logic_pixel: Size,
  pub camera_proj: Box<dyn Projection<f32>>,
  pub camera_world: Mat4<f64>,
}

impl SceneRayQuery {
  pub fn compute_local_tolerance(
    &self,
    tolerance: IntersectTolerance,
    target_world_mat: Mat4<f64>,
    target_object_center_in_world: Vec3<f64>,
  ) -> f32 {
    let target_scale = target_world_mat.max_scale();
    // todo, should we considering camera scale??
    let mut local_tolerance = tolerance.value / target_scale as f32;

    if let ToleranceType::ScreenSpace = tolerance.ty {
      let camera_to_target = target_object_center_in_world - self.world_ray.origin;
      let projected_distance = camera_to_target.dot(target_world_mat.forward());
      let pixel_per_unit = self.camera_proj.pixels_per_unit(
        projected_distance as f32,
        self.camera_view_size_in_logic_pixel.height_usize() as f32,
      );
      local_tolerance /= pixel_per_unit;
    }

    local_tolerance
  }
}

pub trait SceneModelPicker {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>>;
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    for provider in self {
      if let Some(hit) = provider.query(idx, ctx) {
        return Some(hit);
      }
    }
    None
  }
}

pub struct SceneModelPickerBaseImpl<T> {
  pub node_world: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub internal: T,
}

impl<T: LocalModelPicker> SceneModelPicker for SceneModelPickerBaseImpl<T> {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

    if !self.internal.bounding_pre_test(idx, ctx)? {
      return None;
    }

    let mat = self.node_world.access(&node)?;
    let local_ray = ctx
      .world_ray
      .apply_matrix_into(mat.inverse_or_identity())
      .into_f32();

    let hit = self.internal.query_local(idx, ctx, local_ray, mat)?;

    let position = hit.hit.position.into_f64();
    let world_hit_position = position.apply_matrix_into(mat);

    MeshBufferHitPoint {
      hit: HitPoint {
        position: world_hit_position,
        distance: ctx.world_ray.origin.distance_to(world_hit_position),
      },
      primitive_index: hit.primitive_index,
    }
    .into()
  }
}

pub trait LocalModelPicker {
  /// return if intersect with bounding
  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<bool>;

  /// should return hit result in local space
  fn query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_ray: Ray3<f32>,
    target_world: Mat4<f64>,
  ) -> Option<MeshBufferHitPoint>;
}

impl LocalModelPicker for Vec<Box<dyn LocalModelPicker>> {
  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<bool> {
    for provider in self {
      if let Some(hit) = provider.bounding_pre_test(idx, ctx) {
        return Some(hit);
      }
    }
    None
  }

  fn query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_ray: Ray3<f32>,
    target_world: Mat4<f64>,
  ) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) = provider.query_local(idx, ctx, local_ray, target_world) {
        return Some(hit);
      }
    }
    None
  }
}

pub struct AttributeMeshPicker {
  pub sm_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub model_access_std_model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  pub std_model_access_mesh: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  pub mesh_vertex_refs: BoxedDynMultiQuery<RawEntityHandle, RawEntityHandle>,
  pub vertex_buffer_ref: ForeignKeyReadView<SceneBufferViewBufferId<AttributeVertexRef>>,
  pub semantic: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,
  pub mesh_index_attribute: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
  pub mesh_topology: ComponentReadView<AttributesMeshEntityTopology>,
  pub buffer: ComponentReadView<BufferEntityData>,
  pub pick_line_tolerance: IntersectTolerance,
  pub pick_point_tolerance: IntersectTolerance,
}

impl LocalModelPicker for AttributeMeshPicker {
  fn bounding_pre_test(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<bool> {
    let mesh_world_bounding = self.sm_bounding.access(&idx)?;
    IntersectAble::<_, bool, _>::intersect(&ctx.world_ray, &mesh_world_bounding, &()).into()
  }

  fn query_local(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    local_ray: Ray3<f32>,
    target_world: Mat4<f64>,
  ) -> Option<MeshBufferHitPoint> {
    struct PositionBuffer<'a> {
      buffer: &'a [Vec3<f32>],
    }

    impl IndexGet for PositionBuffer<'_> {
      type Output = Vec3<f32>;

      fn index_get(&self, key: usize) -> Option<Self::Output> {
        self.buffer.get(key).copied()
      }
    }

    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;

    let mode = self.mesh_topology.get_value(mesh)?;

    let mut position: Option<&ExternalRefPtr<Vec<u8>>> = None;
    for att in self.mesh_vertex_refs.access_multi(&mesh.into_raw())? {
      let att = unsafe { EntityHandle::from_raw(att) };
      if let AttributeSemantic::Positions = self.semantic.get_value(att).unwrap() {
        let p = self.vertex_buffer_ref.get(att).unwrap();
        position = Some(self.buffer.get(p).unwrap());
      }
    }
    let position = position.unwrap();
    let position = PositionBuffer {
      buffer: bytemuck::cast_slice(position.as_slice()),
    };
    let mut count = position.buffer.len();

    let index = self.mesh_index_attribute.get(mesh).and_then(|v| {
      let buffer = self.buffer.get(v)?;

      if buffer.len() % 4 != 0 {
        let index: &[u16] = cast_slice(buffer);
        count = buffer.len() / 2;
        DynIndexRef::Uint16(index)
      } else {
        let index: &[u32] = cast_slice(buffer);
        count = buffer.len() / 4;
        DynIndexRef::Uint32(index)
      }
      .into()
    });

    let target_world_center = self.sm_bounding.access(&idx)?.center();

    let config = MeshBufferIntersectConfig {
      line_tolerance_local: ctx.compute_local_tolerance(
        self.pick_line_tolerance,
        target_world,
        target_world_center,
      ),
      point_tolerance_local: ctx.compute_local_tolerance(
        self.pick_line_tolerance,
        target_world,
        target_world_center,
      ),
      triangle_face: FaceSide::Double,
    };

    *AttributesMeshEntityAbstractMeshReadView {
      mode,
      vertices: position,
      indices: index,
      count: count / mode.stride(),
    }
    .intersect_nearest(local_ray, &config)
  }
}
