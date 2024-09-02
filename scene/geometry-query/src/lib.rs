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
  pub camera_view_size: Size,
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
  pub model_access_std_model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  pub std_model_access_mesh: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  pub mesh_vertex_refs:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub vertex_buffer_ref: ForeignKeyReadView<SceneBufferViewBufferId<AttributeVertexRef>>,
  pub semantic: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,
  pub mesh_index_attribute: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
  pub mesh_topology: ComponentReadView<AttributesMeshEntityTopology>,
  pub buffer: ComponentReadView<BufferEntityData>,

  pub node_world: Box<dyn DynVirtualCollection<EntityHandle<SceneNodeEntity>, Mat4<f32>>>,
  pub node_net_visible: Box<dyn DynVirtualCollection<EntityHandle<SceneNodeEntity>, bool>>,
}

impl SceneModelPicker for SceneModelPickerImpl {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint> {
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

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

    let model = self.model_access_std_model.get(idx)?;
    let mesh = self.std_model_access_mesh.get(model)?;

    let mode = self.mesh_topology.get_value(mesh)?;

    let mut position: Option<&ExternalRefPtr<Vec<u8>>> = None;
    for att in self.mesh_vertex_refs.access_multi(&mesh)? {
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
      count = buffer.len() / 4; // todo u16 index support
      IndexBuffer {
        buffer: bytemuck::cast_slice(buffer.as_slice()),
      }
      .into()
    });

    let mat = self.node_world.access(&node)?;
    let local_ray = ctx.world_ray.apply_matrix_into(mat.inverse_or_identity());

    AttributesMeshEntityAbstractMeshReadView {
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
