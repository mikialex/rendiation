use crate::*;

pub fn transform_instance_buffer(
  scope: impl ReactiveCollection<AllocIdx<TransformInstancedSceneMesh>, ()>,
) -> impl ReactiveCollection<AllocIdx<TransformInstancedSceneMesh>, GPUBufferResourceView> {

  //       let instance_gpu = create_gpu_buffer(
  //         bytemuck::cast_slice(mesh.transforms.as_slice()),
  //         BufferUsages::VERTEX,
  //         &ctx.gpu.device,
  //       )
  //       .create_default_view();
}

pub struct TransformInstanceGPU<'a> {
  instance: &'a TransformInstancedSceneMesh,
}

only_vertex!(TransformInstanceMat, Mat4<f32>);

#[repr(C)]
#[derive(Clone, Copy, rendiation_shader_api::ShaderVertex)]
pub struct ShaderMat4VertexInput {
  #[semantic(TransformInstanceMat)]
  mat: Mat4<f32>,
}

impl<'a> GraphicsShaderProvider for TransformInstanceGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.mesh_gpu.build(builder)?;
    builder.vertex(|builder, _| {
      builder.register_vertex::<ShaderMat4VertexInput>(VertexStepMode::Instance);

      let world_mat = builder.query::<TransformInstanceMat>()?;
      let world_normal_mat = world_mat.shrink_to_3();

      if let Ok(position) = builder.query::<GeometryPosition>() {
        builder.register::<GeometryPosition>((world_mat * (position, val(1.)).into()).xyz());
      }

      if let Ok(normal) = builder.query::<GeometryNormal>() {
        builder.register::<GeometryNormal>(world_normal_mat * normal);
      }

      Ok(())
    })
  }
}

impl<'a> ShaderHashProvider for TransformInstanceGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.mesh_gpu.hash_pipeline(hasher)
  }
}

impl<'a> ShaderPassBuilder for TransformInstanceGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.instance_gpu);
  }
}

impl<'a> MeshDrawcallEmitter for TransformInstanceGPU<'a> {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    let mut c = self.instance.mesh.draw_command(group);

    let instance_count = self.instance.transforms.len();

    match &mut c {
      DrawCommand::Indexed { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..instance_count
      }
      DrawCommand::Array { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..instance_count
      }
      DrawCommand::Skip => {}
      DrawCommand::MultiIndirect { .. } => {
        panic!("indirect draw is impossible in the transform instance")
      }
    }
    c
  }
}
