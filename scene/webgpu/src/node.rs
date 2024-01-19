use crate::*;

pub fn node_gpus(
  node_mats: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  cx: &ResourceGPUCtx,
) -> impl ReactiveCollection<NodeIdentity, NodeGPU> {
  let uniforms = node_mats.collective_map(|mat| TransformGPUData {
    world_matrix: mat,
    normal_matrix: mat.to_normal_matrix().into(),
    ..Zeroable::zeroed()
  });

  let cx = cx.clone();
  uniforms.collective_execute_map_by(move || {
    let cx = cx.clone();
    move |_, _| {
      let gpu = NodeGPU::new(&cx.device);
      // gpu.update
      gpu
    }
  })
}

#[derive(Clone)]
pub struct NodeGPU {
  pub ubo: UniformBufferDataView<TransformGPUData>,
}

impl PartialEq for NodeGPU {
  fn eq(&self, other: &Self) -> bool {
    false
  }
}

impl std::fmt::Debug for NodeGPU {
  fn fmt(&self, f: &mut __core::fmt::Formatter<'_>) -> __core::fmt::Result {
    f.debug_struct("NodeGPU").finish()
  }
}

impl NodeGPU {
  pub fn update(&mut self, queue: &GPUQueue, world_mat: Mat4<f32>) -> &mut Self {
    self.ubo.set(TransformGPUData::from_world_mat(world_mat));
    self.ubo.upload_with_diff(queue);
    self
  }

  pub fn new(device: &GPUDevice) -> Self {
    let ubo = create_uniform(TransformGPUData::default(), device);
    Self { ubo }
  }

  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<TransformGPUData>> {
    builder
      .bind_by(&self.ubo)
      .using_graphics_pair(builder, |r, node| {
        let node = node.load().expand();
        r.register_typed_both_stage::<WorldMatrix>(node.world_matrix);
        r.register_typed_both_stage::<WorldNormalMatrix>(node.normal_matrix);
      })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl TransformGPUData {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for NodeGPU {}

impl GraphicsShaderProvider for NodeGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let model = binding.bind_by(&self.ubo).load().expand();
      let position = builder.query::<GeometryPosition>()?;
      let position = model.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(model.world_matrix);
      builder.register::<WorldNormalMatrix>(model.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>()?;
      builder.register::<WorldVertexNormal>(model.normal_matrix * normal);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for NodeGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo);
  }
}
