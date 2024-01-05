use crate::*;

pub struct TransformInstanceGPU {
  mesh_gpu: Box<MeshGPUInstance>,
  instance_gpu: GPUBufferResourceView,
  transforms_count: u32,
}

impl Stream for TransformInstanceGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

only_vertex!(TransformInstanceMat, Mat4<f32>);

#[repr(C)]
#[derive(Clone, Copy, rendiation_shader_api::ShaderVertex)]
pub struct ShaderMat4VertexInput {
  #[semantic(TransformInstanceMat)]
  mat: Mat4<f32>,
}

impl GraphicsShaderProvider for TransformInstanceGPU {
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

impl ShaderHashProvider for TransformInstanceGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.mesh_gpu.hash_pipeline(hasher)
  }
}

impl ShaderPassBuilder for TransformInstanceGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.instance_gpu);
  }
}

impl ReactiveRenderComponentSource for TransformInstanceGPUReactive {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl MeshDrawcallEmitter for TransformInstanceGPUReactive {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    let inner: &TransformInstanceGPU = self.inner.as_ref();
    let mut c = inner.mesh_gpu.draw_command(group);

    match &mut c {
      DrawCommand::Indexed { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..inner.transforms_count
      }
      DrawCommand::Array { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..inner.transforms_count
      }
      DrawCommand::Skip => {}
      DrawCommand::MultiIndirect { .. } => {
        panic!("indirect draw is impossible in the transform instance")
      }
    }
    c
  }
}

#[pin_project::pin_project]
pub struct TransformInstanceGPUReactive {
  #[pin]
  pub inner: TransformInstanceGPUReactiveInner,
}

impl Stream for TransformInstanceGPUReactive {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

pub type TransformInstanceGPUReactiveInner =
  impl AsRef<RenderComponentCell<TransformInstanceGPU>> + Stream<Item = RenderComponentDeltaFlag>;

impl WebGPUMesh for TransformInstancedSceneMesh {
  type ReactiveGPU = TransformInstanceGPUReactive;

  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let ctx = ctx.clone();

    let create = move |m: &IncrementalSignalPtr<Self>| {
      let mesh = m.read();
      // todo, current we do not support reuse this inner mesh!
      let mesh_gpu = mesh.mesh.create_scene_reactive_gpu(&ctx).unwrap();
      let mesh_gpu = Box::new(mesh_gpu);

      let instance_gpu = create_gpu_buffer(
        bytemuck::cast_slice(mesh.transforms.as_slice()),
        BufferUsages::VERTEX,
        &ctx.gpu.device,
      )
      .create_default_view();

      let transforms_count = mesh.transforms.len() as u32;

      TransformInstanceGPU {
        mesh_gpu,
        instance_gpu,
        transforms_count,
      }
    };

    let state = RenderComponentCell::new(create(source));

    let inner = source
      .single_listen_by::<()>(any_change_no_init)
      .filter_map_sync(source.defer_weak())
      .fold_signal(state, move |mesh, state| {
        state.inner = create(&mesh);
        RenderComponentDeltaFlag::all().into()
      });

    TransformInstanceGPUReactive { inner }
  }
}
