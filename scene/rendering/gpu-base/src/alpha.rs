use crate::*;

pub struct ShaderAlphaConfig {
  pub alpha_mode: AlphaMode,
  pub alpha_cutoff: Node<f32>,
  pub alpha: Node<f32>,
}

impl ShaderAlphaConfig {
  pub fn apply(&self, builder: &mut ShaderFragmentBuilderView) {
    match self.alpha_mode {
      AlphaMode::Opaque => {}
      AlphaMode::Mask => {
        let alpha = self
          .alpha
          .less_than(self.alpha_cutoff)
          .select(val(0.), self.alpha);
        builder.register::<AlphaChannel>(alpha);
        builder.register::<AlphaCutChannel>(self.alpha_cutoff);
      }
      AlphaMode::Blend => {
        builder.register::<AlphaChannel>(self.alpha);
        builder.frag_output.iter_mut().for_each(|p| {
          if p.is_blendable() {
            p.states.blend = BlendState::ALPHA_BLENDING.into();
          }
        });
      }
    };
  }
}

pub fn material_enabled_alpha_blending<S: AlphaInfoSemantic>(
) -> impl ReactiveQuery<Key = EntityHandle<S::Entity>, Value = bool> {
  global_watch()
    .watch::<AlphaModeOf<S>>()
    .collective_map(|mode| mode == AlphaMode::Blend)
}

pub fn all_kinds_of_materials_enabled_alpha_blending(
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = bool> {
  let sg = material_enabled_alpha_blending::<PbrSGMaterialAlphaConfig>()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<StandardModelRefPbrSGMaterial>());

  let mr = material_enabled_alpha_blending::<PbrMRMaterialAlphaConfig>()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<StandardModelRefPbrMRMaterial>());

  let unlit = material_enabled_alpha_blending::<UnlitMaterialAlphaConfig>()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<StandardModelRefUnlitMaterial>());

  sg.collective_select(mr)
    .collective_select(unlit)
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
}
