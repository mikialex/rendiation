use std::sync::Arc;

use database::*;
use rendiation_algebra::*;
use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_infinity_primitive::{InfinityShaderPlaneEffect, PLANE_DRAW_CMD};
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod filter_fill_clip;
pub use filter_fill_clip::*;

pub fn register_clipping_plane_array_data_model() {
  global_database()
    .declare_entity::<ClippingPlaneEntity>()
    .declare_component::<ClippingPlaneInfo>()
    .declare_foreign_key::<ClippingPlaneRefScene>();

  global_entity_of::<AttributesMeshEntity>().declare_component::<AttributeMeshIsSolid>();
  global_entity_of::<SceneModelEntity>().declare_component::<ClippingPlaneSceneModelSkip>();
}

declare_entity!(
  /// A clipping plane.
  ///
  /// Each plane is associated with a [SceneEntity] through [ClippingPlaneRefScene].
  ///
  /// The renderer clips all content in a scene by every clipping plane associated with that scene.
  ClippingPlaneEntity
);
declare_component!(
  /// Stores the plane as a [Vec4] `[normal.x, normal.y, normal.z, constant]` in world space.
  ClippingPlaneInfo, ClippingPlaneEntity, Vec4<f32>);
declare_foreign_key!(ClippingPlaneRefScene, ClippingPlaneEntity, SceneEntity);

declare_component!(
  /// Marks this [AttributesMeshEntity] as part of a "solid" object, meaning a water-tight manifold.
  /// Note that a single mesh itself does not need to be solid, as long as multiple meshes
  /// can be combined into a logical solid group.
  ///
  /// Implementation note: the attribute mesh is currently the only supported type; we are
  /// considering moving this flag to the scene model level in the future.
  AttributeMeshIsSolid, AttributesMeshEntity, bool, false);

declare_component!(
  /// Marks this [SceneModelEntity] as exempt from clipping by any clipping planes, as an escape hatch.
  ClippingPlaneSceneModelSkip, SceneModelEntity, bool, false);

pub fn use_array_plane_clipping(
  cx: &mut QueryGPUHookCx,
  enable: bool,
  fill_face: bool,
) -> Option<ClippingPlaneArrayRenderer> {
  let (cx, planes_gpu) = cx.use_storage_buffer("gpu clipping planes", 128, u32::MAX);

  cx.use_changes::<ClippingPlaneInfo>()
    .update_storage_array(cx, planes_gpu, 0);

  planes_gpu.use_max_item_count_by_db_entity::<ClippingPlaneEntity>(cx);
  planes_gpu.use_update(cx);

  let config = MultiAccessGPUDataBuilderInit {
    max_possible_many_count: u32::MAX,
    max_possible_one_count: u32::MAX,
    init_many_count_capacity: 16 * 8,
    init_one_count_capacity: 16,
  };

  let updates = cx.use_db_rev_ref_tri_view::<ClippingPlaneRefScene>();
  let planes_gpu_access =
    use_multi_access_gpu(cx, &config, updates, "clipping plane array of scenes");

  let planes_host_access = cx.use_db_rev_ref_typed::<ClippingPlaneRefScene>();

  let (cx, skip_clip) = cx.use_storage_buffer::<Bool>("scene model skip clip", 128, u32::MAX);

  cx.use_changes::<ClippingPlaneSceneModelSkip>()
    .map_changes(Bool::from)
    .update_storage_array(cx, skip_clip, 0);

  skip_clip.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);
  skip_clip.use_update(cx);

  cx.when_render(|| ClippingPlaneArrayRenderer {
    fill_face,
    enable,
    planes_gpu: planes_gpu.get_gpu_buffer(),
    skip_clip: skip_clip.get_gpu_buffer(),
    planes_gpu_access: planes_gpu_access.unwrap(),
    planes_host: read_global_db_component::<ClippingPlaneInfo>(),
    planes_host_access: planes_host_access.expect_resolve_stage(),
  })
}

pub struct ClippingPlaneArrayRenderer {
  pub fill_face: bool,
  pub enable: bool,
  pub planes_gpu: AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
  pub skip_clip: AbstractReadonlyStorageBuffer<[Bool]>,
  pub planes_gpu_access: MultiAccessGPUData,
  pub planes_host: ComponentReadView<ClippingPlaneInfo>,
  pub planes_host_access: RevRefForeignKeyReadTyped<ClippingPlaneRefScene>,
}

impl ClippingPlaneArrayRenderer {
  pub fn fill_face(&self, scene: EntityHandle<SceneEntity>) -> bool {
    self.enable && self.fill_face && self.planes_host_access.access_multi(&scene).is_some()
  }

  pub fn use_get_scene_clipping<'a>(
    &'a self,
    scene_id: EntityHandle<SceneEntity>,
    frame_ctx: &mut FrameCtx,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    if !self.enable {
      return None;
    }

    // todo cache
    let scene_id = create_uniform(
      Vec4::new(scene_id.alloc_index(), 0, 0, 0),
      &frame_ctx.gpu.device,
      "scene id",
    );

    Some(Box::new(ClipComponent {
      planes_gpu: &self.planes_gpu,
      planes_gpu_access: &self.planes_gpu_access,
      scene_id,
      ty: ClipDrawType::MainPass,
      skip_clip: &self.skip_clip,
    }))
  }
}

pub struct PlaneCapDrawer<'a> {
  pub writer: &'a dyn RenderComponent,
  pub clip: &'a ClipComponent<'a>,
  pub plane: &'a InfinityShaderPlaneEffect<'a>,
  pub material_injector: &'a MaterialInjector,
  pub lighting: &'a dyn RenderComponent,
  pub reversed_depth: bool,
}

impl PassContent for PlaneCapDrawer<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth).disable_auto_write();
    let com: [&dyn RenderComponent; _] = [
      &base,
      self.plane,
      self.writer,
      self.clip,
      self.lighting,
      self.material_injector,
    ];
    let com = RenderArray(com);

    com.render(&mut pass.ctx, PLANE_DRAW_CMD)
  }
}

pub struct MaterialInjector {}

impl ShaderHashProvider for MaterialInjector {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for MaterialInjector {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      builder.insert_type_tag::<LightableSurfaceTag>();
      // enable blend for transparent face fill
      builder.frag_output.iter_mut().for_each(|p| {
        if p.is_blendable() {
          p.states.blend = BlendState::ALPHA_BLENDING.into();
        }
      });
    })
  }
}

impl ShaderPassBuilder for MaterialInjector {}

pub struct ClipComponent<'a> {
  pub planes_gpu: &'a AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
  pub skip_clip: &'a AbstractReadonlyStorageBuffer<[Bool]>,
  pub planes_gpu_access: &'a MultiAccessGPUData,
  pub scene_id: UniformBufferDataView<Vec4<u32>>,
  pub ty: ClipDrawType,
}

pub enum ClipDrawType {
  MainPass,
  PlaneScenePass(UniformBufferDataView<Vec4<u32>>),
  PlaneSelf(UniformBufferDataView<Vec4<u32>>),
}

impl<'a> ShaderHashProvider for ClipComponent<'a> {
  shader_hash_type_id!(ClipComponent<'static>);
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(std::mem::discriminant(&self.ty));
  }
}

impl<'a> GraphicsShaderProvider for ClipComponent<'a> {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if let ClipDrawType::PlaneScenePass(_) = &self.ty {
      builder.vertex(|builder, _| {
        builder.primitive_state.cull_mode = None;
      });
    }

    builder.fragment(|builder, binding| {
      let planes_gpu_access = self.planes_gpu_access.build(binding);
      let planes_gpu = binding.bind_by(self.planes_gpu);
      let scene_id = binding.bind_by(&self.scene_id).load().x();
      let iter = planes_gpu_access.iter_refed_many_of(scene_id);

      let fragment_render =
        builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      // todo, support high precision
      let position = builder.query::<CameraWorldPositionHP>().expand().f1 + fragment_render;

      let skip_clip = binding.bind_by(self.skip_clip);

      match &self.ty {
        ClipDrawType::MainPass => {
          let sm_id =
            builder.query_or_interpolate_by::<LogicalRenderEntityId, LogicalRenderEntityId>();
          let can_clip = skip_clip.index(sm_id).load().into_bool().not();

          iter.for_each(|clip_id, _cx| {
            let plane = planes_gpu.index(clip_id).load();

            let should_clip = (position.dot(plane.xyz()) + plane.w()).greater_than(val(0.));
            if_by(should_clip.and(can_clip), || {
              builder.discard();
            });
          });
        }
        ClipDrawType::PlaneScenePass(self_plane_id) => {
          let sm_id =
            builder.query_or_interpolate_by::<LogicalRenderEntityId, LogicalRenderEntityId>();
          let can_clip = skip_clip.index(sm_id).load().into_bool().not();

          let self_plane_id = binding.bind_by(self_plane_id).load().x();
          iter.for_each(|clip_id, _cx| {
            // todo, this is not optimal
            if_by(self_plane_id.equals(clip_id), || {
              let plane = planes_gpu.index(clip_id).load();
              let should_clip = (position.dot(plane.xyz()) + plane.w()).greater_than(val(0.));
              if_by(should_clip.and(can_clip), || {
                builder.discard();
              });
            });
          });

          let depth_stencil = builder.depth_stencil.as_mut().unwrap();

          depth_stencil.stencil.read_mask = 0xffffffff;
          depth_stencil.stencil.write_mask = 0xffffffff;

          depth_stencil.stencil.front.compare = CompareFunction::Always;
          depth_stencil.stencil.front.pass_op = StencilOperation::DecrementWrap;
          depth_stencil.stencil.front.fail_op = StencilOperation::DecrementWrap;
          depth_stencil.stencil.front.depth_fail_op = StencilOperation::DecrementWrap;

          depth_stencil.stencil.back.compare = CompareFunction::Always;
          depth_stencil.stencil.back.pass_op = StencilOperation::IncrementWrap;
          depth_stencil.stencil.back.fail_op = StencilOperation::IncrementWrap;
          depth_stencil.stencil.back.depth_fail_op = StencilOperation::IncrementWrap;
        }
        ClipDrawType::PlaneSelf(self_plane_id) => {
          let self_plane_id = binding.bind_by(self_plane_id).load().x();
          iter.for_each(|clip_id, _cx| {
            if_by(self_plane_id.not_equals(clip_id), || {
              let plane = planes_gpu.index(clip_id).load();
              let should_clip = (position.dot(plane.xyz()) + plane.w()).greater_than(val(0.));
              if_by(should_clip, || {
                builder.discard();
              });
            });
          });

          let depth_stencil = builder.depth_stencil.as_mut().unwrap();

          depth_stencil.stencil.read_mask = 0xffffffff;
          depth_stencil.stencil.write_mask = 0xffffffff;
          depth_stencil.stencil.front.compare = CompareFunction::Equal;
          depth_stencil.stencil.back.compare = CompareFunction::Equal;
        }
      }
    });
  }
}

impl<'a> ShaderPassBuilder for ClipComponent<'a> {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.planes_gpu_access.bind(&mut ctx.binding);
    ctx.binding.bind(self.planes_gpu);
    ctx.binding.bind(&self.scene_id);
    ctx.binding.bind(self.skip_clip);

    match &self.ty {
      ClipDrawType::MainPass => {}
      ClipDrawType::PlaneScenePass(self_plane_id) => {
        ctx.binding.bind(self_plane_id);
      }
      ClipDrawType::PlaneSelf(self_plane_id) => {
        ctx.binding.bind(self_plane_id);
        ctx.pass.set_stencil_reference(1);
      }
    }
  }
}

/// todo, fill cap pick is not correctly supported
pub fn use_array_clip_pick_filter(cx: &mut impl DBHookCxLike) -> Option<ArrayClipPickFilter> {
  let scene_ref_clip_planes = cx
    .use_db_rev_ref::<ClippingPlaneRefScene>()
    .use_assure_result(cx);

  cx.when_resolve_stage(move || {
    let sm_skip_clip = read_global_db_component::<ClippingPlaneSceneModelSkip>();
    let scene_ref_clip_planes = scene_ref_clip_planes.expect_resolve_stage();
    let planes = read_global_db_component::<ClippingPlaneInfo>();

    ArrayClipPickFilter {
      sm_skip_clip,
      scene_ref_clip_planes,
      planes,
    }
  })
}

pub struct ArrayClipPickFilter {
  sm_skip_clip: ComponentReadView<ClippingPlaneSceneModelSkip>,
  scene_ref_clip_planes: RevRefForeignKeyRead,
  planes: ComponentReadView<ClippingPlaneInfo>,
}

impl ArrayClipPickFilter {
  pub fn create_filter(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> impl Fn(&MeshBufferHitPoint<f64>, EntityHandle<SceneModelEntity>) -> bool + '_ {
    move |hit_point: &MeshBufferHitPoint<f64>, sm_id| {
      if self.sm_skip_clip.get(sm_id) == Some(&true) {
        return true;
      }
      let mut should_clip = false;

      if let Some(iter) = self.scene_ref_clip_planes.access_multi(&scene.into_raw()) {
        for plane in iter {
          let plane = unsafe { EntityHandle::from_raw(plane) };
          let plane = self.planes.get(plane).unwrap();
          if hit_point.hit.position.into_f32().dot(plane.xyz()) + plane.w() > 0. {
            should_clip = true;
            break;
          }
        }
      }

      !should_clip
    }
  }
}
