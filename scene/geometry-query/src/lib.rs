use database::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

pub struct SceneRayQuery {
  pub world_ray: Ray3,
  pub conf: MeshBufferIntersectConfig,
  pub node_world: Box<dyn DynVirtualCollection<u32, Mat4<f32>>>,
  pub node_visible: Box<dyn DynVirtualCollection<u32, bool>>,
  pub model_lookup:
    Box<dyn DynVirtualMultiCollection<EntityHandle<SceneEntity>, EntityHandle<SceneModelEntity>>>,
  pub camera_view_size: Size,

  pub scene_model_picker: Vec<Box<dyn SceneModelPicker>>,
}

pub trait SceneModelPicker {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint>;
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) = provider.query(idx, ctx) {
        return Some(hit);
      }
    }
    None
  }
}

pub struct SceneModelPickerImpl {
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub model_access_std_model: Box<
    dyn DynVirtualCollection<EntityHandle<SceneModelEntity>, EntityHandle<StandardModelEntity>>,
  >,
  pub std_model_access_mesh: Box<
    dyn DynVirtualCollection<EntityHandle<StandardModelEntity>, EntityHandle<AttributeMeshEntity>>,
  >,
  pub mesh_position_attribute:
    Box<dyn DynVirtualCollection<EntityHandle<AttributeMeshEntity>, EntityHandle<BufferEntity>>>,
  pub mesh_index_attribute:
    Box<dyn DynVirtualCollection<EntityHandle<AttributeMeshEntity>, EntityHandle<BufferEntity>>>,
  pub mesh_topology: ComponentReadView<AttributeMeshTopology>,
  pub buffer: ComponentReadView<BufferEntityData>,
}

impl SceneModelPicker for SceneModelPickerImpl {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint> {
    struct PositionBuffer<'a> {
      buffer: &'a [Vec3<f32>],
    }

    impl<'a> IndexGet for PositionBuffer<'a> {
      type Output = Vec3<f32>;

      fn index_get(&self, key: usize) -> Option<Self::Output> {
        self.buffer.get(key).copied()
      }
    }

    struct IndexBuffer<'a> {
      buffer: &'a [u32],
    }

    impl<'a> IndexGet for IndexBuffer<'a> {
      type Output = usize;

      fn index_get(&self, key: usize) -> Option<Self::Output> {
        self.buffer.get(key).map(|v| *v as usize)
      }
    }

    let model = self.model_access_std_model.access(&idx)?;
    let mesh = self.std_model_access_mesh.access(&model)?;

    let mode = self.mesh_topology.get_value(mesh)?;

    let position = self.mesh_position_attribute.access(&mesh)?;
    let position = self.buffer.get(position)?;
    let position = PositionBuffer {
      buffer: bytemuck::cast_slice(position.as_slice()),
    };
    let mut count = position.buffer.len();

    let index = self.mesh_position_attribute.access(&mesh).and_then(|v| {
      let buffer = self.buffer.get(v)?;
      count = buffer.len() / 4;
      IndexBuffer {
        buffer: bytemuck::cast_slice(buffer.as_slice()),
      }
      .into()
    });

    let node = self.scene_model_node.get(idx)?;
    let mat = ctx.node_world.access(&node.alloc_index())?;
    let local_ray = ctx.world_ray.apply_matrix_into(mat.inverse_or_identity());

    AttributeMeshAbstractMeshReadView {
      mode,
      vertices: position,
      indices: index,
      count: count / mode.stride(),
      draw_count: count,
    }
    .intersect_nearest(local_ray, &ctx.conf, MeshGroup { start: 0, count })
    .0
  }
}

impl SceneRayQuery {
  pub fn query(&self, scene: EntityHandle<SceneEntity>) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    for idx in self.model_lookup.access_multi_value(&scene) {
      if let Some(hit) = self.scene_model_picker.query(idx, self) {
        nearest.refresh(hit);
      }
    }

    nearest
  }
}
