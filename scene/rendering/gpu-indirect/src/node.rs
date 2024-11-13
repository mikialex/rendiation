use crate::*;

both!(IndirectSceneNodeId, u32);
pub type SceneNodeStorages = ReactiveStorageBufferContainer<NodeStorage>;

pub fn node_storages(cx: &GPU) -> SceneNodeStorages {
  let source = scene_node_derive_world_mat().collective_map(|mat| NodeStorage {
    world_matrix: mat,
    normal_matrix: mat.to_normal_matrix().into(),
    ..Zeroable::zeroed()
  });

  SceneNodeStorages::new(cx).with_source(source, 0)
}

pub struct NodeGPUStorage<'a> {
  pub buffer: &'a MultiUpdateContainer<CommonStorageBufferImpl<NodeStorage>>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct NodeStorage {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl NodeStorage {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl<'a> ShaderHashProvider for NodeGPUStorage<'a> {
  shader_hash_type_id! {NodeGPUStorage<'static>}
}

impl<'a> GraphicsShaderProvider for NodeGPUStorage<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let nodes = binding.bind_by(self.buffer.inner.gpu());
      let current_node_id = builder.query::<IndirectSceneNodeId>();
      let node = nodes.index(current_node_id).load().expand();

      let position = builder.query::<GeometryPosition>();
      let position = node.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(node.world_matrix);
      builder.register::<WorldNormalMatrix>(node.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>();
      builder.register::<WorldVertexNormal>(node.normal_matrix * normal);
    })
  }
}

impl<'a> ShaderPassBuilder for NodeGPUStorage<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
