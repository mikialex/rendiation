use rendiation_infinity_primitive::InfinityShaderPlaneEffect;

use crate::*;

pub fn register_clipping_plane_array_data_model() {
  global_database()
    .declare_entity::<ClippingPlaneEntity>()
    .declare_component::<ClippingPlaneInfo>()
    .declare_foreign_key::<ClippingPlaneRefScene>();

  global_entity_of::<AttributesMeshEntity>().declare_component::<AttributeMeshIsSolid>();
  global_entity_of::<SceneModelEntity>().declare_component::<ClippingPlaneSceneModelSkip>();
}

declare_entity!(ClippingPlaneEntity);
declare_component!(ClippingPlaneInfo, ClippingPlaneEntity, Vec4<f32>);
declare_foreign_key!(ClippingPlaneRefScene, ClippingPlaneEntity, SceneEntity);

declare_component!(AttributeMeshIsSolid, AttributesMeshEntity, bool, false);
declare_component!(ClippingPlaneSceneModelSkip, SceneModelEntity, bool, false);

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
  fill_face: bool,
  enable: bool,
  planes_gpu: AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
  skip_clip: AbstractReadonlyStorageBuffer<[Bool]>,
  planes_gpu_access: MultiAccessGPUData,
  planes_host: ComponentReadView<ClippingPlaneInfo>,
  planes_host_access: RevRefForeignKeyReadTyped<ClippingPlaneRefScene>,
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
    );

    Some(Box::new(ClipComponent {
      planes_gpu: &self.planes_gpu,
      planes_gpu_access: &self.planes_gpu_access,
      scene_id,
      ty: ClipDrawType::MainPass,
      skip_clip: &self.skip_clip,
    }))
  }

  pub fn use_fill_surface(
    &self,
    frame_ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    g_buffer: &FrameGeometryBuffer,
    target: ClipFillType,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &CameraGPU,
    scene: EntityHandle<SceneEntity>,
    lighting_sys: &SceneLightSystem,
    filter: &IsSolidFilter,
  ) {
    let reverse_z = renderer.reversed_depth;
    let mut all_object = renderer.batch_extractor.extract_scene_batch(
      scene,
      SceneContentKey::default(),
      renderer.scene,
    );
    filter.install_filter(&mut all_object);
    // todo flush filter reduce filter cost

    let planes = self.planes_host_access.access_multi(&scene);

    // todo cache
    let scene_id = create_uniform(
      Vec4::new(scene.alloc_index(), 0, 0, 0),
      &frame_ctx.gpu.device,
    );

    let fmt = match g_buffer.depth.format() {
      TextureFormat::Depth16Unorm => TextureFormat::Depth24PlusStencil8,
      TextureFormat::Depth24Plus => TextureFormat::Depth24PlusStencil8,
      TextureFormat::Depth24PlusStencil8 => TextureFormat::Depth24PlusStencil8,
      TextureFormat::Depth32Float => {
        if frame_ctx
          .gpu
          .info()
          .supported_features
          .contains(Features::DEPTH32FLOAT_STENCIL8)
        {
          TextureFormat::Depth32FloatStencil8
        } else {
          TextureFormat::Depth24PlusStencil8
        }
      }
      TextureFormat::Depth32FloatStencil8 => TextureFormat::Depth32FloatStencil8,
      _ => unreachable!("expect depth fmt"),
    };

    let temp_depth_stencil = depth_attachment().format(fmt).request(frame_ctx);

    let m_buffer = FrameGeneralMaterialBuffer::new(frame_ctx);

    if let Some(planes) = planes {
      if self.enable && self.fill_face {
        frame_ctx.next_key_scope_root();
        for plane in planes {
          frame_ctx.keyed_scope(&plane, |frame_ctx| {
            let plane_id = create_uniform(
              Vec4::new(plane.alloc_index(), 0, 0, 0),
              &frame_ctx.gpu.device,
            );
            let clip = ClipComponent {
              planes_gpu: &self.planes_gpu,
              planes_gpu_access: &self.planes_gpu_access,
              ty: ClipDrawType::PlaneScenePass(plane_id.clone()),
              scene_id: scene_id.clone(),
              skip_clip: &self.skip_clip,
            };

            let mut pass_base = pass("clip per plane boundary extract").with_depth(
              &temp_depth_stencil,
              clear_and_store(if reverse_z { 0. } else { 0. }),
              clear_and_store(0),
            );

            let indices = m_buffer.extend_pass_desc(&mut pass_base);
            let material_writer = FrameGeneralMaterialBufferEncoder {
              indices,
              materials: &lighting_sys.system.material_defer_lighting_supports,
            };

            let clip_dispatcher = RenderArray([
              &clip as &dyn RenderComponent,
              &material_writer,
              &DisableAllChannelBlend,
            ]);

            // todo, try move out side
            let mut content = renderer.scene.make_scene_batch_pass_content(
              all_object.clone(),
              camera_gpu,
              &clip_dispatcher,
              frame_ctx,
            );

            pass_base.render_ctx(frame_ctx).by(&mut content);

            ////

            let plane = self.planes_host.get(plane).unwrap();
            // todo cache
            let plane = create_uniform_with_cache(
              rendiation_shader_library::plane::ShaderPlaneUniform::new(
                plane.xyz().into_f64(),
                plane.w() as f64,
              ),
              &frame_ctx.gpu.device,
            );
            let plane = InfinityShaderPlaneEffect {
              plane: &plane,
              camera: camera_gpu,
              reversed_depth: reverse_z,
            };

            let clip = ClipComponent {
              planes_gpu: &self.planes_gpu,
              planes_gpu_access: &self.planes_gpu_access,
              ty: ClipDrawType::PlaneSelf(plane_id),
              scene_id: scene_id.clone(),
              skip_clip: &self.skip_clip,
            };

            let material_buffer = FrameGeneralMaterialBufferReconstructSurface {
              m_buffer: &m_buffer,
              registry: &lighting_sys.system.material_defer_lighting_supports,
            };

            match target {
              ClipFillType::Forward {
                scene_result,
                forward_lighting: _,
              } => {
                let mut pass_base = pass("draw clip plane").with_depth(
                  &temp_depth_stencil,
                  load_and_store(),
                  load_and_store(),
                );
                let color_writer = DefaultDisplayWriter::extend_pass_desc(
                  &mut pass_base,
                  scene_result,
                  load_and_store(),
                );
                // todo, write g buffer entity id buffer(if exist)

                let lighting = lighting_sys.get_scene_lighting_component(
                  scene,
                  camera,
                  Box::new(DirectGeometryProvider),
                  &material_buffer,
                );

                let mut filler = PlaneCapDrawer {
                  writer: &color_writer,
                  clip: &clip,
                  plane: &plane,
                  material_injector: &MaterialInjector {},
                  lighting: &lighting,
                  reversed_depth: reverse_z,
                };

                pass_base.render_ctx(frame_ctx).by(&mut filler);
              }
              ClipFillType::Defer(_frame_general_material_buffer) => todo!(),
            }
          })
        }
      }
    }
  }
}

struct PlaneCapDrawer<'a> {
  writer: &'a dyn RenderComponent,
  clip: &'a ClipComponent<'a>,
  plane: &'a InfinityShaderPlaneEffect<'a>,
  material_injector: &'a MaterialInjector,
  lighting: &'a dyn RenderComponent,
  reversed_depth: bool,
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

    com.render(&mut pass.ctx, rendiation_infinity_primitive::PLANE_DRAW_CMD)
  }
}

struct MaterialInjector {}

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

struct ClipComponent<'a> {
  planes_gpu: &'a AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
  skip_clip: &'a AbstractReadonlyStorageBuffer<[Bool]>,
  planes_gpu_access: &'a MultiAccessGPUData,
  scene_id: UniformBufferDataView<Vec4<u32>>,
  ty: ClipDrawType,
}

enum ClipDrawType {
  MainPass,
  PlaneScenePass(UniformBufferDataView<Vec4<u32>>),
  PlaneSelf(UniformBufferDataView<Vec4<u32>>),
}

impl<'a> ShaderHashProvider for ClipComponent<'a> {
  shader_hash_type_id!(ClipComponent<'static>);
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(&self.ty).hash(hasher);
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
