use crate::*;

pub fn use_viewer_std_model_renderer(
  cx: &mut QueryGPUHookCx,
  materials: Option<Box<dyn IndirectModelMaterialRenderImpl>>,
  shapes: Option<Box<dyn IndirectModelShapeRenderImpl>>,
  revere_z: bool,
) -> Option<SceneStdModelIndirectRenderer> {
  let material_flat = cx.use_changes::<StandardModelRefUnlitMaterial>();
  let material_pbr_mr = cx.use_changes::<StandardModelRefPbrMRMaterial>();
  let material_pbr_sg = cx.use_changes::<StandardModelRefPbrSGMaterial>();
  let material_occ = cx.use_changes::<StdModelOccStyleMaterialPayload>();

  let material_key = if cx.is_spawning_stage() {
    let material_flat = material_flat.into_spawn_stage_ready();
    let material_pbr_mr = material_pbr_mr.into_spawn_stage_ready();
    let material_pbr_sg = material_pbr_sg.into_spawn_stage_ready();
    let material_occ = material_occ.into_spawn_stage_ready();

    let mut r = Vec::new();
    if let Some(v) = material_flat {
      r.push(v.map_some_u32_index());
    }
    if let Some(v) = material_pbr_mr {
      r.push(v.map_some_u32_index());
    }
    if let Some(v) = material_pbr_sg {
      r.push(v.map_some_u32_index());
    }
    if let Some(v) = material_occ {
      r.push(v.map_some_u32_index());
    }
    UseResult::SpawnStageReady(SelectChanges(r))
  } else {
    UseResult::NotInStage
  };

  use_std_model_renderer(cx, materials, material_key, shapes, revere_z)
}
