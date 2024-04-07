use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: Texture2DHandle,
  pub sampler: SamplerHandle,
  pub sys: GPUTextureBindingSystem,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.sys.bind_texture(&mut ctx.binding, self.texture);
    self.sys.bind_sampler(&mut ctx.binding, self.sampler);
  }

  pub fn bind_and_sample(
    &self,
    binding: &mut ShaderBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    handles: Node<TextureSamplerHandlePair>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let handles = handles.expand();
    self.sys.maybe_sample_texture2d_indirect_and_bind_shader(
      binding,
      reg,
      (self.texture, self.sampler),
      (handles.texture_handle, handles.sampler_handle),
      uv,
    )
  }

  pub fn bind_and_sample_enabled(
    &self,
    binding: &mut ShaderBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    handles: Node<TextureSamplerHandlePair>,
    uv: Node<Vec2<f32>>,
  ) -> (Node<Vec4<f32>>, Node<bool>) {
    let handles = handles.expand();
    let r = self.sys.maybe_sample_texture2d_indirect_and_bind_shader(
      binding,
      reg,
      (self.texture, self.sampler),
      (handles.texture_handle, handles.sampler_handle),
      uv,
    );
    (r, handles.texture_handle.equals(val(0)))
  }
}

impl ShareBindableResourceCtx {
  pub fn build_reactive_texture_sampler_pair(
    &self,
    t: Option<&Texture2DWithSamplingData>,
  ) -> ReactiveGPUTextureSamplerPair {
    let (sampler_changes, _) = t
      .map(|t| self.get_or_create_reactive_gpu_sampler(&t.sampler))
      .unwrap_or(self.get_or_create_reactive_gpu_sampler(&self.default_sampler));

    let (texture_changes, _) = t
      .map(|t| self.get_or_create_reactive_gpu_texture2d(&t.texture))
      .unwrap_or(self.get_or_create_reactive_gpu_texture2d(&self.default_texture_2d));

    let pair = GPUTextureSamplerPair {
      texture: 0, // will be updated later
      sampler: 0, // will be updated later
      sys: self.binding_sys.clone(),
    };

    let sampler_changes = sampler_changes.filter_map_sync(|v| match v {
      BindableGPUChange::ReferenceSampler(_, v) => ContentOrHandleChange::Handle(v).into(),
      BindableGPUChange::Content => ContentOrHandleChange::Content.into(),
      _ => None,
    });
    let texture_changes = texture_changes.filter_map_sync(|v| match v {
      BindableGPUChange::Reference2D(_, v) => ContentOrHandleChange::Handle(v).into(),
      BindableGPUChange::Content => ContentOrHandleChange::Content.into(),
      _ => None,
    });

    ReactiveGPUTextureSamplerPair {
      pair,
      texture_changes,
      sampler_changes,
    }
  }
}

pub enum ContentOrHandleChange {
  Content,
  Handle(u32),
}

pub type GPUTexture2dHandleChange = impl Stream<Item = ContentOrHandleChange> + Unpin;
pub type GPUSamplerHandleChange = impl Stream<Item = ContentOrHandleChange> + Unpin;

#[pin_project::pin_project]
pub struct ReactiveGPUTextureSamplerPair {
  pair: GPUTextureSamplerPair,
  #[pin]
  texture_changes: GPUTexture2dHandleChange,
  #[pin]
  sampler_changes: GPUSamplerHandleChange,
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
  RefChange(MaybeDelta<TextureSamplerHandlePair>),
}

impl ReactiveGPUTextureSamplerPair {
  pub fn poll_change(
    &mut self,
    cx: &mut Context,
    flag: &mut RenderComponentDeltaFlag,
    cb: impl FnMut(TextureSamplerHandlePairDelta),
  ) {
    if let Poll::Ready(Some(change)) = self.poll_next_unpin(cx) {
      *flag |= RenderComponentDeltaFlag::Content;
      if let ReactiveGPUTextureSamplerPairDelta::RefChange(change) = change {
        *flag |= RenderComponentDeltaFlag::ContentRef;
        change.expand_delta(cb);
      }
    }
  }
}
impl Stream for ReactiveGPUTextureSamplerPair {
  type Item = ReactiveGPUTextureSamplerPairDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let texture = this.texture_changes.poll_next(cx);
    let sampler = this.sampler_changes.poll_next(cx);

    use ContentOrHandleChange::*;
    use ReactiveGPUTextureSamplerPairDelta::*;
    use TextureSamplerHandlePairDelta::*;

    if let Poll::Ready(Some(Handle(t))) = &texture {
      this.pair.texture = *t;
    }
    if let Poll::Ready(Some(Handle(t))) = &sampler {
      this.pair.sampler = *t;
    }

    // is this too complicated?
    match (texture, sampler) {
      (Poll::Ready(Some(t)), Poll::Ready(Some(s))) => Poll::Ready(Some(match (t, s) {
        (Handle(th), Handle(sh)) => RefChange(MaybeDelta::All(TextureSamplerHandlePair {
          texture_handle: th,
          sampler_handle: sh,
          ..Default::default()
        })),
        (Content, Content) => ContentChange,
        (Content, Handle(h)) => RefChange(MaybeDelta::Delta(sampler_handle(h))),
        (Handle(h), Content) => RefChange(MaybeDelta::Delta(texture_handle(h))),
      })),
      (Poll::Ready(Some(r)), Poll::Pending) => Poll::Ready(Some(match r {
        Content => ContentChange,
        Handle(h) => RefChange(MaybeDelta::Delta(texture_handle(h))),
      })),
      (Poll::Pending, Poll::Ready(Some(r))) => Poll::Ready(Some(match r {
        Content => ContentChange,
        Handle(h) => RefChange(MaybeDelta::Delta(sampler_handle(h))),
      })),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None),
    }
  }
}

impl Deref for ReactiveGPUTextureSamplerPair {
  type Target = GPUTextureSamplerPair;
  fn deref(&self) -> &Self::Target {
    &self.pair
  }
}
