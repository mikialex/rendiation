use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

pub struct FlatMaterialGPU {
  uniform: UniformBufferDataView<FlatMaterialUniform>,
}

impl Stream for FlatMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl ShaderHashProvider for FlatMaterialGPU {}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
  }
}

impl ReactiveRenderComponentSource for ReactiveMaterialGPUOf<FlatMaterial> {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl WebGPUMaterial for FlatMaterial {
  type ReactiveGPU =
    impl AsRef<RenderComponentCell<FlatMaterialGPU>> + Stream<Item = RenderComponentDeltaFlag>;

  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let uniform = FlatMaterialUniform {
      color: source.read().color,
      ..Zeroable::zeroed()
    };
    let uniform = create_uniform2(uniform, &ctx.gpu.device);

    let gpu = FlatMaterialGPU { uniform };
    let state = RenderComponentCell::new(gpu);

    let weak_material = source.downgrade();
    let ctx = ctx.clone();

    source
      .single_listen_by::<()>(any_change_no_init)
      .fold_signal(state, move |_, state| {
        if let Some(m) = weak_material.upgrade() {
          let uniform = FlatMaterialUniform {
            color: m.read().color,
            ..Zeroable::zeroed()
          };
          state.inner.uniform.resource.set(uniform);
          state.inner.uniform.resource.upload(&ctx.gpu.queue);
        }
        RenderComponentDeltaFlag::Content.into()
      })
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}
