use rendiation_color::*;

use crate::*;

/// just shortcut to avoid write closure
#[inline(always)]
pub fn then_some<X>(f: impl Fn(X) -> bool) -> impl Fn(X) -> Option<()> {
  move |d| f(d).then_some(())
}

pub fn apply_normal_map_delta(
  t: DeltaOf<Option<NormalMapping>>,
  target: &mut ReactiveGPUTextureSamplerPair,
  ctx: &ShareBindableResourceCtx,
) -> RenderComponentDeltaFlag {
  if let Some(t) = t {
    match t {
      MaybeDelta::Delta(v) => match v {
        NormalMappingDelta::content(t) => {
          *target = ctx.build_reactive_texture_sampler_pair(Some(&t));
          RenderComponentDeltaFlag::RefAndHash
        }
        // scale handled in uniform groups
        NormalMappingDelta::scale(_) => RenderComponentDeltaFlag::Content,
      },
      MaybeDelta::All(t) => {
        *target = ctx.build_reactive_texture_sampler_pair(Some(&t.content));
        RenderComponentDeltaFlag::RefAndHash
      }
    }
  } else {
    *target = ctx.build_reactive_texture_sampler_pair(None);
    RenderComponentDeltaFlag::RefAndHash
  }
}

pub fn apply_tex_pair_delta(
  t: Option<MaybeDelta<TextureWithSamplingData<IncrementalSignalPtr<SceneTexture2DType>>>>,
  target: &mut ReactiveGPUTextureSamplerPair,
  ctx: &ShareBindableResourceCtx,
) -> RenderComponentDeltaFlag {
  *target = ctx.build_reactive_texture_sampler_pair(t.map(merge_maybe).as_ref());
  RenderComponentDeltaFlag::RefAndHash
}

pub enum UniformChangePicked<T> {
  UniformChange,
  Origin(T),
}

pub fn srgba_to_linear(color: Vec4<f32>) -> Vec4<f32> {
  let alpha = color.a();
  let color = srgb_to_linear(color.rgb());
  Vec4::new(color.x, color.y, color.z, alpha)
}

pub fn srgb_to_linear(color: Vec3<f32>) -> Vec3<f32> {
  let color: SRGBColor<f32> = color.into();
  let color: LinearRGBColor<f32> = color.into();
  color.into()
}
