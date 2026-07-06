#![feature(impl_trait_in_assoc_type)]

use database::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;

mod draw;
use draw::*;

mod pick;
pub use pick::*;

mod gles_draw;
pub use gles_draw::*;

mod indirect_draw;
pub use indirect_draw::*;

pub fn register_wide_line_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelWideLineRenderPayload>(sparse);

  global_database()
    .declare_entity::<WideLineModelEntity>()
    .declare_component::<WideLineWidth>()
    .declare_component::<WideLineColor>()
    .declare_component::<WideLineStylePattern>()
    .declare_component::<WideLineStyleFactor>()
    .declare_component::<WideLineEnableRoundJoint>()
    .declare_component::<WideLineDepthEnable>()
    .declare_component::<WideLineTransparent>()
    .declare_component::<WideLineMeshBuffer>();
}

declare_foreign_key!(
  SceneModelWideLineRenderPayload,
  SceneModelEntity,
  WideLineModelEntity
);

declare_entity!(WideLineModelEntity);
declare_component!(WideLineWidth, WideLineModelEntity, f32, 1.0);
declare_component!(
  WideLineColor,
  WideLineModelEntity,
  Vec4<f32>,
  Vec4::new(1.0, 1.0, 1.0, 1.0)
);

declare_component!(WideLineTransparent, WideLineModelEntity, bool, false);
declare_component!(WideLineDepthEnable, WideLineModelEntity, bool, true);
declare_component!(WideLineStyleFactor, WideLineModelEntity, f32, 1.0);
declare_component!(WideLineStylePattern, WideLineModelEntity, u32, 0);
declare_component!(WideLineEnableRoundJoint, WideLineModelEntity, bool, false);

declare_component!(
  WideLineMeshBuffer,
  WideLineModelEntity,
  ExternalRefPtr<Vec<WideLineVertex>>
);

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
#[derive(Facet, Serialize, Deserialize)]
pub struct WideLineVertex {
  #[semantic(WideLineStart)]
  pub start: Vec3<f32>,
  #[semantic(WideLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

/// the one_pixel_native_line_optimization_enabled must be immutable for every call
pub fn use_wide_line_vertices_count(
  cx: &mut impl DBHookCxLike,
  one_pixel_native_line_optimization_enabled: bool,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = u32>> {
  let wide_line_v_count = cx
    .use_dual_query::<WideLineMeshBuffer>()
    .dual_query_zip(cx.use_dual_query::<WideLineWidth>())
    .dual_query_map(move |(v, width)| {
      let line_seg_count = v.len() as u32;
      if one_pixel_native_line_optimization_enabled && width == 1.0 {
        line_seg_count * 2
      } else {
        line_seg_count * 18
      }
    });

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelWideLineRenderPayload>();
  wide_line_v_count.fanout(relation, cx)
}
