use std::sync::Arc;

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

pub struct TransparentHostOrderer {
  world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3>,
}

impl TransparentHostOrderer {
  pub fn reorder_content(
    &self,
    content: &dyn HostRenderBatch,
    camera_position: Vec3<f32>,
  ) -> Box<dyn HostRenderBatch> {
    let mut content = content
      .iter_scene_models()
      .map(|sm| {
        let distance = if let Some(bounding) = self.world_bounding.access(&sm) {
          bounding.center().distance2_to(camera_position)
        } else {
          0.
        };
        (distance, sm)
      })
      .collect::<Vec<_>>();
    content.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    Box::new(DistanceReorderedHostRenderBatch {
      internal: Arc::new(content),
    })
  }
}

#[derive(Clone)]
struct DistanceReorderedHostRenderBatch {
  internal: Arc<Vec<(f32, EntityHandle<SceneModelEntity>)>>,
}

impl HostRenderBatch for DistanceReorderedHostRenderBatch {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.internal.as_slice().iter().map(|(_, sm)| *sm))
  }
}
