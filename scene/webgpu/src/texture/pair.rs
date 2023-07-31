use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: GPU2DTextureView,
  pub sampler: GPUSamplerView,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.texture);
    ctx.binding.bind(&self.sampler);
  }

  pub fn uniform_and_sample(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    handles: Node<TextureSamplerHandlePair>,
    position: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let _handles = handles.expand();
    let texture = binding.uniform_by(&self.texture);
    let sampler = binding.uniform_by(&self.sampler);
    texture.sample(sampler, position)
  }

  pub fn uniform_and_sample_enabled(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    handles: Node<TextureSamplerHandlePair>,
    position: Node<Vec2<f32>>,
  ) -> (Node<Vec4<f32>>, Node<bool>) {
    let handles = handles.expand();
    let texture = binding.uniform_by(&self.texture);
    let sampler = binding.uniform_by(&self.sampler);
    (
      texture.sample(sampler, position),
      handles.texture_handle.equals(consts(0)),
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn build_reactive_texture_sampler_pair(
    &self,
    t: Option<&Texture2DWithSamplingData>,
  ) -> ReactiveGPUTextureSamplerPair {
    let ReactiveGPUSamplerView {
      gpu: sampler,
      changes: sampler_changes,
    } = t
      .map(|t| self.get_or_create_reactive_gpu_sampler(&t.sampler))
      .unwrap_or(self.get_or_create_reactive_gpu_sampler(&self.default_sampler));

    let ReactiveGPU2DTextureView {
      gpu: texture,
      changes,
    } = t
      .map(|t| self.get_or_create_reactive_gpu_texture2d(&t.texture))
      .unwrap_or(self.get_or_create_reactive_gpu_texture2d(&self.default_texture_2d));

    let pair = GPUTextureSamplerPair { texture, sampler };

    ReactiveGPUTextureSamplerPair {
      pair,
      changes,
      sampler_changes,
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveGPUTextureSamplerPair {
  pair: GPUTextureSamplerPair,
  #[pin]
  changes: Texture2dRenderComponentDeltaStream,
  #[pin]
  sampler_changes: SamplerRenderComponentDeltaStream,
}
#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Incremental, ShaderStruct, Default)]
pub struct TextureSamplerHandlePair {
  pub texture_handle: u32,
  pub sampler_handle: u32,
}

pub enum ReactiveGPUTextureSamplerPairDelta {
  ContentChange,
  RefChange(DeltaOf<TextureSamplerHandlePair>),
}

impl ReactiveGPUTextureSamplerPair {
  pub fn poll_change(
    &mut self,
    cx: &mut Context,
    flag: &mut RenderComponentDeltaFlag,
    cb: impl FnOnce(TextureSamplerHandlePairDelta),
  ) {
    if let Poll::Ready(Some(change)) = self.poll_next_unpin(cx) {
      *flag |= RenderComponentDeltaFlag::Content;
      if let ReactiveGPUTextureSamplerPairDelta::RefChange(change) = change {
        *flag |= RenderComponentDeltaFlag::ContentRef;
        cb(change);
      }
    }
  }
}
impl Stream for ReactiveGPUTextureSamplerPair {
  type Item = ReactiveGPUTextureSamplerPairDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    todo!()
    // let this = self.project();
    // let texture = this.changes.poll_next(cx);
    // let sampler = this.sampler_changes.poll_next(cx);
    // match (texture, sampler) {
    //   (Poll::Ready(t), Poll::Ready(s)) => match (t, s) {
    //     (Some(t), Some(s)) => Poll::Ready(Some(t | s)),
    //     _ => Poll::Ready(None),
    //   },
    //   (Poll::Ready(r), Poll::Pending) => Poll::Ready(r),
    //   (Poll::Pending, Poll::Ready(r)) => Poll::Ready(r),
    //   (Poll::Pending, Poll::Pending) => Poll::Pending,
    // }
  }
}

impl Deref for ReactiveGPUTextureSamplerPair {
  type Target = GPUTextureSamplerPair;
  fn deref(&self) -> &Self::Target {
    &self.pair
  }
}

pub struct GPUTextureSamplerProxyPair {
  pub texture: Texture2DHandle,
  pub sampler: SamplerHandle,
}

#[pin_project::pin_project]
pub struct ReactiveGPUTextureSamplerProxyPair {
  pair: GPUTextureSamplerProxyPair,
  #[pin]
  changes: Texture2dRenderComponentDeltaStream,
  #[pin]
  sampler_changes: SamplerRenderComponentDeltaStream,
}
