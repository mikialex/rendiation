use crate::*;

pub struct TransformInstance<M> {
  pub mesh: M,
  pub transforms: Vec<Mat4<f32>>,
}

pub struct TransformInstanceGPU<M: WebGPUMesh> {
  mesh_gpu: M::GPU,
  instance_gpu: Rc<webgpu::Buffer>,
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

      let world_mat = builder.query::<TransformInstanceMat>()?.get();
      let world_normal_mat: Node<Mat3<f32>> = world_mat.into();
      let world_normal_mat = world_normal_mat.inverse().transpose();

      if let Ok(position) = builder.query::<GeometryPosition>() {
        position.set(world_mat * position.get())
      }

      if let Ok(normal) = builder.query::<GeometryNormal>() {
        normal.set(world_normal_mat * normal.get())
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

impl<M: WebGPUMesh> WebGPUMesh for TransformInstance<M> {
  type GPU = TransformInstanceGPU<M>;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) -> Self::GPU {
    let mesh_gpu = self.mesh.create(gpu, storage);
    let instance_gpu = gpu
      .device
      .deref()
      .create_buffer_init(&webgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(self.transforms.as_slice()),
        usage: BufferUsages::VERTEX,
      });
    TransformInstanceGPU {
      mesh_gpu,
      instance_gpu: Rc::new(instance_gpu),
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
    }
    inner
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.mesh.topology()
  }

  fn try_pick(
    &self,
    _f: &mut dyn FnMut(&dyn rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh),
  ) {
    // todo support picking?
  }
}

// impl<M: WebGPUMesh> WebGPUSceneMesh for Identity<TransformInstance<M>> {
//   fn check_update_gpu<'a>(
//     &self,
//     res: &'a mut GPUMeshCache,
//     sub_res: &mut AnyMap,
//     gpu: &GPU,
//   ) -> &'a dyn RenderComponentAny {
//     let type_id = TypeId::of::<StencilFaceState>();

//     // let mapper = self
//     //   .inner
//     //   .entry(type_id)
//     //   .or_insert_with(|| Box::new(MeshIdentityMapper::<M>::default()))
//     //   .downcast_mut::<MeshIdentityMapper<M>>()
//     //   .unwrap();
//     // mapper.get_update_or_insert_with_logic(m, |x| match x {
//     //   ResourceLogic::Create(m) => ResourceLogicResult::Create(m.create(gpu, storage)),
//     //   ResourceLogic::Update(gpu_m, m) => {
//     //     m.update(gpu_m, gpu, storage);
//     //     ResourceLogicResult::Update(gpu_m)
//     //   }
//     // })
//   }

//   fn topology(&self) -> webgpu::PrimitiveTopology {
//     WebGPUMesh::topology(self)
//   }

//   fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
//     WebGPUMesh::draw_impl(self, group)
//   }
// }
