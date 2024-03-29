use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

pub struct FlatMaterialGPU {
  uniform: UniformBufferCachedDataView<FlatMaterialUniform>,
}

impl Stream for FlatMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl ShaderHashProvider for FlatMaterialGPU {}

impl GraphicsShaderProvider for FlatMaterialGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
  }
}

#[pin_project::pin_project]
pub struct FlatMaterialReactiveGPU {
  #[pin]
  pub inner: FlatMaterialReactiveGPUImpl,
}

impl Stream for FlatMaterialReactiveGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for FlatMaterialReactiveGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

pub type FlatMaterialReactiveGPUImpl =
  impl AsRef<RenderComponentCell<FlatMaterialGPU>> + Stream<Item = RenderComponentDeltaFlag>;

impl WebGPUMaterial for FlatMaterial {
  type ReactiveGPU = FlatMaterialReactiveGPU;

  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let uniform = create_flat_material_uniform(&source.read());
    let uniform = create_uniform_with_cache(uniform, &ctx.gpu.device);

    let gpu = FlatMaterialGPU { uniform };
    let state = RenderComponentCell::new(gpu);

    let ctx = ctx.clone();

    let inner = source
      .single_listen_by::<()>(any_change_no_init)
      .filter_map_sync(source.defer_weak())
      .fold_signal(state, move |m, state| {
        let uniform = create_flat_material_uniform(&m.read());
        state.inner.uniform.set(uniform);
        state.inner.uniform.upload(&ctx.gpu.queue);
        RenderComponentDeltaFlag::Content.into()
      });

    FlatMaterialReactiveGPU { inner }
  }

  fn is_transparent(&self) -> bool {
    false
  }
}

fn create_flat_material_uniform(m: &FlatMaterial) -> FlatMaterialUniform {
  FlatMaterialUniform {
    color: srgba_to_linear(m.color),
    ..Zeroable::zeroed()
  }
}
