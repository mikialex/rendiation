use rendiation_infinity_primitive::InfinityShaderPlaneEffect;

use crate::*;

pub fn register_clipping_plane_array_data_model() {
  global_database()
    .declare_entity::<ClippingPlaneEntity>()
    .declare_component::<ClippingPlaneInfo>()
    .declare_foreign_key::<ClippingPlaneRefScene>();

  global_entity_of::<AttributesMeshEntity>().declare_component::<AttributeMeshIsSolid>();
}

declare_entity!(ClippingPlaneEntity);
declare_component!(ClippingPlaneInfo, ClippingPlaneEntity, Vec4<f32>);
declare_foreign_key!(ClippingPlaneRefScene, ClippingPlaneEntity, SceneEntity);

declare_component!(AttributeMeshIsSolid, AttributesMeshEntity, bool, false);

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

  cx.when_render(|| ClippingPlaneArrayRenderer {
    fill_face,
    enable,
    planes_gpu: planes_gpu.get_gpu_buffer(),
    planes_gpu_access: planes_gpu_access.unwrap(),
    planes_host: read_global_db_component::<ClippingPlaneInfo>(),
    planes_host_access: planes_host_access.expect_resolve_stage(),
  })
}

pub struct ClippingPlaneArrayRenderer {
  fill_face: bool,
  enable: bool,
  planes_gpu: AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
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
    }))
  }

  pub fn use_fill_surface(
    &self,
    frame_ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    g_buffer: &FrameGeometryBuffer,
    target: ClipFillType,
    camera_gpu: &CameraGPU,
    scene: EntityHandle<SceneEntity>,
  ) {
    let reverse_z = renderer.reversed_depth;
    let all_object = renderer.batch_extractor.extract_scene_batch(
      scene,
      SceneContentKey::default(),
      renderer.scene,
    );

    let planes = self.planes_host_access.access_multi(&scene);

    // todo cache
    let scene_id = create_uniform(
      Vec4::new(scene.alloc_index(), 0, 0, 0),
      &frame_ctx.gpu.device,
    );

    let temp_depth = attachment()
      .format(TextureFormat::Depth32FloatStencil8)
      .request(frame_ctx);

    let depth_clear = if reverse_z {
      clear_and_store(0.)
    } else {
      clear_and_store(1.)
    };

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
            };

            // todo, try move out side
            let mut content = renderer.scene.make_scene_batch_pass_content(
              all_object.clone(),
              camera_gpu,
              &clip,
              frame_ctx,
            );

            assert!(g_buffer.depth.format().has_stencil_aspect());

            pass("clip per plane boundary extract")
              .with_depth(&g_buffer.depth, depth_clear, clear_and_store(0))
              .render_ctx(frame_ctx)
              .by(&mut content);

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
            };

            let temp_texture_raw = temp_depth.expect_texture_view::<f32>();
            frame_ctx.encoder.copy_texture_to_texture(
              TexelCopyTextureInfo {
                texture: temp_texture_raw.texture(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::StencilOnly,
              },
              TexelCopyTextureInfo {
                texture: g_buffer.depth.expect_texture_view::<f32>().texture(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::StencilOnly,
              },
              temp_texture_raw.size().into_gpu_size(),
            );

            match target {
              ClipFillType::Forward {
                scene_result,
                forward_lighting,
              } => {
                let mut pass_base = pass("draw clip plane");
                let color_writer = DefaultDisplayWriter::extend_pass_desc(
                  &mut pass_base,
                  scene_result,
                  load_and_store(),
                );
                // todo, write g buffer entity id buffer(if exist)

                let mut filler = FillFace {
                  writer: &color_writer,
                  clip: &clip,
                  plane: &plane,
                  material_injector: &MaterialInjector {},
                  lighting: forward_lighting,
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

struct FillFace<'a> {
  pub writer: &'a dyn RenderComponent,
  pub clip: &'a ClipComponent<'a>,
  pub plane: &'a InfinityShaderPlaneEffect<'a>,
  pub material_injector: &'a MaterialInjector,
  pub lighting: &'a dyn RenderComponent,
}

impl PassContent for FillFace<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let com: [&dyn RenderComponent; _] = [
      self.writer,
      self.plane,
      self.clip,
      self.material_injector,
      self.lighting,
    ];
    let com = RenderArray(com);

    com.render(&mut pass.ctx, rendiation_infinity_primitive::PLANE_DRAW_CMD)
  }
}

struct MaterialInjector {
  //
}

impl ShaderHashProvider for MaterialInjector {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for MaterialInjector {}

impl ShaderPassBuilder for MaterialInjector {}

struct ClipComponent<'a> {
  planes_gpu: &'a AbstractReadonlyStorageBuffer<[Vec4<f32>]>,
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

      match &self.ty {
        ClipDrawType::MainPass => {
          iter.for_each(|clip_id, _cx| {
            let plane = planes_gpu.index(clip_id).load();

            let should_clip = (position.dot(plane.xyz()) + plane.w()).greater_than(val(0.));
            if_by(should_clip, || {
              builder.discard();
            });
          });
        }
        ClipDrawType::PlaneScenePass(self_plane_id) => {
          let self_plane_id = binding.bind_by(self_plane_id).load().x();
          iter.for_each(|clip_id, _cx| {
            // todo, this is not optimal
            if_by(self_plane_id.equals(clip_id), || {
              let plane = planes_gpu.index(clip_id).load();
              let should_clip = (position.dot(plane.xyz()) + plane.w()).greater_than(val(0.));
              if_by(should_clip, || {
                builder.discard();
              });
            });
          });

          let depth_stencil = builder.depth_stencil.as_mut().unwrap();
          depth_stencil.stencil.front.compare = CompareFunction::Always;
          depth_stencil.stencil.front.pass_op = StencilOperation::DecrementWrap;

          depth_stencil.stencil.back.compare = CompareFunction::Always;
          depth_stencil.stencil.back.pass_op = StencilOperation::IncrementClamp;
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
