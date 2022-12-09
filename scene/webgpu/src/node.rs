use crate::*;

pub struct NodeGPUStore {
  inner: IdentityMapper<TransformGPU, SceneNodeDataImpl>,
}

impl NodeGPUStore {
  pub fn new(gpu: &GPU) -> Self {
    //
  }
}

impl NodeGPUStore {
  pub fn check_update_gpu(&mut self, n: &SceneNode, cb: &mut dyn FnMut(&TransformGPU)) {
    // n.visit(|node| {
    //   let r = self.get_update_or_insert_with(
    //     node,
    //     |_node| TransformGPU::new(gpu, n, None),
    //     |node_gpu, _node| {
    //       node_gpu.update(gpu, n, None);
    //     },
    //   );

    //   cb(r);
    // })
  }
}

impl std::ops::Deref for NodeGPUStore {
  type Target = IdentityMapper<TransformGPU, SceneNodeDataImpl>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for NodeGPUStore {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

pub struct TransformGPU {
  pub ubo: UniformBufferDataView<TransformGPUData>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader140Mat3,
}

impl TransformGPUData {
  pub fn from_node(node: &SceneNode, override_mat: Option<Mat4<f32>>) -> Self {
    let world_matrix = override_mat.unwrap_or_else(|| node.get_world_matrix());
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for TransformGPU {}

impl ShaderGraphProvider for TransformGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, binding| {
      let model = binding.uniform_by(&self.ubo, SB::Object).expand();
      let position = builder.query::<GeometryPosition>()?;
      let position = model.world_matrix * (position, 1.).into();

      builder.register::<WorldMatrix>(model.world_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>()?;
      builder.register::<WorldVertexNormal>(model.normal_matrix * normal);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for TransformGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo, SB::Object)
  }
}

impl TransformGPU {
  pub fn update(
    &mut self,
    gpu: &GPU,
    node: &SceneNode,
    override_mat: Option<Mat4<f32>>,
  ) -> &mut Self {
    let ubo = &self.ubo.resource;
    ubo.set(TransformGPUData::from_node(node, override_mat));
    ubo.upload_with_diff(&gpu.queue);
    self
  }

  pub fn new(gpu: &GPU, node: &SceneNode, override_mat: Option<Mat4<f32>>) -> Self {
    let ubo = create_uniform(TransformGPUData::default(), gpu);
    ubo
      .resource
      .set(TransformGPUData::from_node(node, override_mat));
    ubo.resource.upload(&gpu.queue);

    Self { ubo }
  }
}
