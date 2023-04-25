use crate::*;

/// just shortcut to avoid write closure
#[inline(always)]
pub fn then_some<X>(f: impl Fn(X) -> bool) -> impl Fn(X) -> Option<()> {
  move |d| f(d).then_some(())
}

pub fn apply_normal_map_delta(
  t: DeltaOf<Option<NormalMapping>>,
  target: &mut Option<ReactiveGPUTextureSamplerPair>,
  ctx: &ShareBindableResourceCtx,
) -> RenderComponentDeltaFlag {
  if let Some(t) = t {
    match t {
      MaybeDelta::Delta(v) => match v {
        NormalMappingDelta::content(t) => {
          *target = ctx.build_reactive_texture_sampler_pair(&t).into();
          RenderComponentDeltaFlag::RefAndHash
        }
        // scale handled in uniform groups
        NormalMappingDelta::scale(_) => RenderComponentDeltaFlag::Content,
      },
      MaybeDelta::All(t) => {
        *target = ctx.build_reactive_texture_sampler_pair(&t.content).into();
        RenderComponentDeltaFlag::RefAndHash
      }
    }
  } else {
    *target = None;
    RenderComponentDeltaFlag::RefAndHash
  }
}

pub fn apply_tex_pair_delta(
  t: Option<MaybeDelta<TextureWithSamplingData<SceneItemRef<SceneTexture2DType>>>>,
  target: &mut Option<ReactiveGPUTextureSamplerPair>,
  ctx: &ShareBindableResourceCtx,
) -> RenderComponentDeltaFlag {
  *target = t
    .map(merge_maybe)
    .map(|t| ctx.build_reactive_texture_sampler_pair(&t));
  RenderComponentDeltaFlag::RefAndHash
}

pub enum UniformChangePicked<T> {
  UniformChange,
  Origin(T),
}
