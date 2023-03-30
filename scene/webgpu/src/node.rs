use crate::*;

pub type NodeGPUMap = ReactiveMap<SceneNode, NodeGPU>;

impl ReactiveMapping<NodeGPU> for SceneNode {
  type ChangeStream = impl Stream + Unpin;
  type DropFuture = impl Future<Output = ()> + Unpin;
  type Ctx<'a> = (&'a GPU, &'a SceneNodeDeriveSystem);

  fn key(&self) -> usize {
    self.id()
  }

  fn build(
    &self,
    (gpu, derives): &Self::Ctx<'_>,
  ) -> (NodeGPU, Self::ChangeStream, Self::DropFuture) {
    let drop = self.visit(|node| node.create_drop());
    let gpu_node = NodeGPU::new(gpu, self, None, derives);
    let change = derives.create_world_matrix_stream(self);
    (gpu_node, change, drop)
  }

  fn update(
    &self,
    gpu_node: &mut NodeGPU,
    change: &mut Self::ChangeStream,
    (gpu, derives): &Self::Ctx<'_>,
  ) {
    do_updates(change, |_| {
      gpu_node.update(gpu, self, None, derives);
    });
  }
}

pub struct NodeGPU {
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
  pub fn from_node(
    node: &SceneNode,
    override_mat: Option<Mat4<f32>>,
    de: &SceneNodeDeriveSystem,
  ) -> Self {
    let world_matrix = override_mat.unwrap_or_else(|| de.get_world_matrix(node));
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for NodeGPU {}

impl ShaderGraphProvider for NodeGPU {
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

impl ShaderPassBuilder for NodeGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo, SB::Object)
  }
}

impl NodeGPU {
  pub fn update(
    &mut self,
    gpu: &GPU,
    node: &SceneNode,
    override_mat: Option<Mat4<f32>>,
    de: &SceneNodeDeriveSystem,
  ) -> &mut Self {
    let ubo = &self.ubo.resource;
    ubo.set(TransformGPUData::from_node(node, override_mat, de));
    ubo.upload_with_diff(&gpu.queue);
    self
  }

  pub fn new(
    gpu: &GPU,
    node: &SceneNode,
    override_mat: Option<Mat4<f32>>,
    de: &SceneNodeDeriveSystem,
  ) -> Self {
    let ubo = create_uniform(TransformGPUData::default(), gpu);
    ubo
      .resource
      .set(TransformGPUData::from_node(node, override_mat, de));
    ubo.resource.upload(&gpu.queue);

    Self { ubo }
  }
}
