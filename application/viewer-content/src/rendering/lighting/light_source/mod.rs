use crate::*;

mod directional;
pub use directional::*;

mod spot;
pub use spot::*;

pub struct ShadowMapPreparerEntry {
  pub preparer: BasicShadowMapPreparer,
  pub shadow_map: Box<dyn AbstractShadowMapGPUData>,
}

pub struct ShadowMapGPUDataEntry {
  pub gpu_data: BasicShadowMapInfoGPU,
  pub shadow_map: Box<dyn AbstractShadowMapGPUData>,
}

mod point;
pub use point::*;

mod area;
pub use area::*;

mod ibl;
pub use ibl::*;

pub fn use_shadow_map(
  cx: &mut QueryGPUHookCx,
  lighting_sys: &LightSystem,
  reversed_depth: bool,
  rebuild: Option<SizeWithDepth>,
) -> Box<dyn AbstractShadowMapGPUData> {
  match lighting_sys.filter_ty {
    ViewerShadowFilterType::PCF => cx.scope(|cx| {
      let (cx, shadow) = cx.use_plain_state(|| PCFShadowMapGPUData {
        atlas: None,
        pcf_config_parameter: create_pcf_parameter(cx.gpu, lighting_sys.pcf_config),
        pcf_config: lighting_sys.pcf_config,
        reversed_depth,
      });

      if cx.is_in_render() {
        // todo diff update
        shadow.pcf_config_parameter = create_pcf_parameter(cx.gpu, lighting_sys.pcf_config);
        shadow.pcf_config = lighting_sys.pcf_config;
      }

      if let Some(rebuild) = rebuild {
        shadow.check_rebuild(rebuild, cx.gpu);
      }

      Box::new(shadow.clone())
    }),
    ViewerShadowFilterType::VSM => cx.scope(|cx| {
      let (cx, shadow) =
        cx.use_plain_state(|| VSMShadowMap::new(lighting_sys.vsm_config, reversed_depth, cx.gpu));

      if cx.is_in_render() {
        shadow.update_config(lighting_sys.vsm_config, cx.gpu);
      }

      if let Some(rebuild) = rebuild {
        shadow.check_rebuild(rebuild, cx.gpu);
      }

      Box::new(shadow.clone())
    }),
  }
}

pub fn use_basic_shadow_map_entry(
  cx: &mut QueryGPUHookCx,
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
  shadow_info: Option<(BasicShadowMapPreparer, SizeWithDepth)>,
) -> Option<ShadowMapPreparerEntry> {
  let shadowmap = use_shadow_map(
    cx,
    lighting_sys,
    ndc.enable_reverse_z,
    shadow_info.as_ref().map(|v| v.1),
  );

  shadow_info.map(|v| ShadowMapPreparerEntry {
    preparer: v.0,
    shadow_map: shadowmap,
  })
}
