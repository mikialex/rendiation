use rendiation_infinity_primitive::InfinityShaderPlaneEffect;
use rendiation_texture_gpu_process::copy_frame;

use crate::*;

pub fn use_fill_surface(
  plane_renderer: &ClippingPlaneArrayRenderer,
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
  let mut all_object =
    renderer
      .batch_extractor
      .extract_scene_batch(scene, SceneContentKey::default(), renderer.scene);
  filter.use_execute(&mut all_object, frame_ctx);

  let planes = plane_renderer.planes_host_access.access_multi(&scene);

  // todo cache
  let scene_id = create_uniform(
    Vec4::new(scene.alloc_index(), 0, 0, 0),
    &frame_ctx.gpu.device,
    "scene id",
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

  frame_ctx.next_scope_index();
  if let Some(planes) = planes
    && plane_renderer.enable
    && plane_renderer.fill_face
  {
    for plane in planes {
      frame_ctx.keyed_scope(&plane, |frame_ctx| {
        let plane_id = create_uniform(
          Vec4::new(plane.alloc_index(), 0, 0, 0),
          &frame_ctx.gpu.device,
          "plane id",
        );
        let clip = ClipComponent {
          planes_gpu: &plane_renderer.planes_gpu,
          planes_gpu_access: &plane_renderer.planes_gpu_access,
          ty: ClipDrawType::PlaneScenePass(plane_id.clone()),
          scene_id: scene_id.clone(),
          skip_clip: &plane_renderer.skip_clip,
        };

        let mut pass_base = pass("clip per plane boundary extract").with_depth(
          &temp_depth_stencil,
          clear_and_store(if reverse_z { 0. } else { 1. }),
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
        let mut content = renderer.scene.use_make_scene_batch_pass_content(
          all_object.clone(),
          camera_gpu,
          &clip_dispatcher,
          frame_ctx,
        );

        pass_base.render_ctx(frame_ctx).by(&mut content);

        ////

        let plane = plane_renderer.planes_host.get(plane).unwrap();
        // todo cache
        let plane = create_uniform_with_cache(
          rendiation_shader_library::plane::ShaderPlaneUniform::new(
            plane.xyz().into_f64(),
            plane.w() as f64,
          ),
          &frame_ctx.gpu.device,
          "plane",
        );
        let plane = InfinityShaderPlaneEffect {
          plane: &plane,
          camera: camera_gpu,
          reversed_depth: reverse_z,
        };

        let clip = ClipComponent {
          planes_gpu: &plane_renderer.planes_gpu,
          planes_gpu_access: &plane_renderer.planes_gpu_access,
          ty: ClipDrawType::PlaneSelf(plane_id),
          scene_id: scene_id.clone(),
          skip_clip: &plane_renderer.skip_clip,
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
            // todo, write g buffer entity id buffer(if exist)
            // through this, we can support clip cap gpu pick.

            let mut key = scene_result.create_attachment_key();
            key.sample_count = 1;
            let color_temp = key.request(frame_ctx);

            let color_writer = DefaultDisplayWriter::extend_pass_desc(
              &mut pass_base,
              &color_temp,
              clear_and_store(all_zero()),
            );

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

            // we do this copy(and the temp color stuff) to support msaa scene target
            // this can be skip if the scene target is not msaa, for simplicity here we always do extra copy.
            pass("copy clip plane cap fill result")
              .with_color(scene_result, load_and_store())
              .render_ctx(frame_ctx)
              .by(&mut copy_frame(
                color_temp,
                Some(BlendState::ALPHA_BLENDING),
              ));
          }
          ClipFillType::Defer(_frame_general_material_buffer) => todo!(),
        }
      })
    }
  }
}
