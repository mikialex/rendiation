use crate::*;

#[derive(Clone)]
pub struct TransformInstance<M> {
  pub mesh: M,
  pub transforms: Vec<Mat4<f32>>,
}

impl<M: Clone + Send + Sync> SimpleIncremental for TransformInstance<M> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

pub struct TransformInstanceGPU<M: WebGPUMesh> {
  mesh_gpu: M::GPU,
  instance_gpu: GPUBufferResourceView,
}

only_vertex!(TransformInstanceMat, Mat4<f32>);

#[repr(C)]
#[derive(Clone, Copy, shadergraph::ShaderVertex)]
pub struct ShaderMat4VertexInput {
  #[semantic(TransformInstanceMat)]
  mat: Mat4<f32>,
}

impl<M: WebGPUMesh> ShaderGraphProvider for TransformInstanceGPU<M> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.mesh_gpu.build(builder)?;
    builder.vertex(|builder, _| {
      builder.register_vertex::<ShaderMat4VertexInput>(VertexStepMode::Instance);

      let world_mat = builder.query::<TransformInstanceMat>()?;
      let world_normal_mat: Node<Mat3<f32>> = world_mat.into();

      if let Ok(position) = builder.query::<GeometryPosition>() {
        builder.register::<GeometryPosition>((world_mat * (position, 1.).into()).xyz());
      }

      if let Ok(normal) = builder.query::<GeometryNormal>() {
        builder.register::<GeometryNormal>(world_normal_mat * normal);
      }

      Ok(())
    })
  }
}

impl<M: WebGPUMesh> ShaderHashProvider for TransformInstanceGPU<M> {}

impl<M: WebGPUMesh> ShaderPassBuilder for TransformInstanceGPU<M> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.instance_gpu);
  }
}

impl<M: WebGPUMesh + Clone> WebGPUMesh for TransformInstance<M> {
  type GPU = TransformInstanceGPU<M>;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) -> Self::GPU {
    let mesh_gpu = self.mesh.create(gpu, storage);

    let instance_gpu = create_gpu_buffer(
      bytemuck::cast_slice(self.transforms.as_slice()),
      webgpu::BufferUsages::VERTEX,
      &gpu.device,
    )
    .create_default_view();

    TransformInstanceGPU {
      mesh_gpu,
      instance_gpu,
    }
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    let mut inner = self.mesh.draw_impl(group);
    match &mut inner {
      DrawCommand::Indexed { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..self.transforms.len() as u32;
      }
      DrawCommand::Array { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..self.transforms.len() as u32;
      }
      DrawCommand::Skip => {}
    }
    inner
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.mesh.topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    self.mesh.try_pick(&mut |target| {
      let wrapped = InstanceTransformedPickImpl {
        mat: &self.transforms,
        mesh: target,
      };
      f(&wrapped as &dyn IntersectAbleGroupedMesh)
    });
  }
}

struct InstanceTransformedPickImpl<'a> {
  pub mat: &'a [Mat4<f32>],
  pub mesh: &'a dyn IntersectAbleGroupedMesh,
}

impl<'a> IntersectAbleGroupedMesh for InstanceTransformedPickImpl<'a> {
  fn intersect_list(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self.mat.iter().for_each(|mat| {
      let world_inv = mat.inverse_or_identity();
      let local_ray = ray.clone().apply_matrix_into(world_inv);
      self.mesh.intersect_list(local_ray, conf, result, group)
    })
  }

  fn intersect_nearest(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self
      .mat
      .iter()
      .fold(OptionalNearest::none(), |mut pre, mat| {
        let world_inv = mat.inverse_or_identity();
        let local_ray = ray.clone().apply_matrix_into(world_inv);
        let r = self.mesh.intersect_nearest(local_ray, conf, group);
        *pre.refresh_nearest(r)
      })
  }
}
