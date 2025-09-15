use std::sync::Arc;

use crate::*;

pub struct ShaderAlphaConfig {
  pub alpha_mode: AlphaMode,
  pub alpha_cutoff: Node<f32>,
  pub alpha: Node<f32>,
}

impl ShaderAlphaConfig {
  /// note, after this call, the uniform control flow is not exist
  pub fn apply(&self, builder: &mut ShaderFragmentBuilderView) {
    match self.alpha_mode {
      AlphaMode::Opaque => {}
      AlphaMode::Mask => {
        let cut = self.alpha.less_than(self.alpha_cutoff);

        if_by(cut, || {
          builder.discard();
        });
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
) -> impl Query<Key = EntityHandle<S::Entity>, Value = bool> {
  get_db_view_typed::<AlphaModeOf<S>>().map_value(|mode| mode == AlphaMode::Blend)
}

pub fn all_kinds_of_materials_enabled_alpha_blending(
) -> impl Query<Key = EntityHandle<SceneModelEntity>, Value = bool> {
  let sg = get_db_view_typed_foreign::<StandardModelRefPbrSGMaterial>()
    .chain(material_enabled_alpha_blending::<PbrSGMaterialAlphaConfig>())
    .into_boxed(); // these boxes can be removed maybe

  let mr = get_db_view_typed_foreign::<StandardModelRefPbrMRMaterial>()
    .chain(material_enabled_alpha_blending::<PbrMRMaterialAlphaConfig>())
    .into_boxed();

  let unlit = get_db_view_typed_foreign::<StandardModelRefUnlitMaterial>()
    .chain(material_enabled_alpha_blending::<UnlitMaterialAlphaConfig>())
    .into_boxed();

  get_db_view_typed_foreign::<SceneModelStdModelRenderPayload>().chain(Select([sg, mr, unlit]))
}

pub struct TransparentHostOrderer {
  pub world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
}

impl TransparentHostOrderer {
  pub fn reorder_content(
    &self,
    content: &dyn HostRenderBatch,
    camera_position: Vec3<f64>,
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

    content.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));

    Box::new(DistanceReorderedHostRenderBatch {
      internal: Arc::new(content),
    })
  }
}

#[derive(Clone)]
struct DistanceReorderedHostRenderBatch {
  internal: Arc<Vec<(f64, EntityHandle<SceneModelEntity>)>>,
}

impl HostRenderBatch for DistanceReorderedHostRenderBatch {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.internal.as_slice().iter().map(|(_, sm)| *sm))
  }
}
