use rendiation_csg_sdf_expression::*;
use rendiation_oit::AtomicImageDowngrade;

use crate::*;

pub fn register_clipping_data_model() {
  register_csg_sdf_data_model();
  global_entity_of::<SceneEntity>().declare_foreign_key::<SceneCSGClipping>();
}

declare_foreign_key!(SceneCSGClipping, SceneEntity, CSGExpressionNodeEntity);

pub fn test_clipping_data(scene: EntityHandle<SceneEntity>) {
  let mut w = global_entity_of::<CSGExpressionNodeEntity>().entity_writer();

  fn write_plane(
    w: &mut EntityWriter<CSGExpressionNodeEntity>,
    dir: Vec3<f32>,
    constant: f32,
  ) -> EntityHandle<CSGExpressionNodeEntity> {
    let plane = Plane::new(dir.into_normalized(), constant);
    let plane = CSGExpressionNode::Plane(plane);
    w.new_entity(|w| w.write::<CSGExpressionNodeContent>(&Some(plane)))
  }

  let p1 = write_plane(&mut w, Vec3::new(1., 0., 0.), 0.);
  let p2 = write_plane(&mut w, Vec3::new(0., 0., 1.), 0.);
  let p3 = write_plane(&mut w, Vec3::new(0., 1., 0.), 0.);

  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
  });
  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&root.some_handle())
      .write::<CSGExpressionRightChild>(&p3.some_handle())
  });

  global_entity_component_of::<SceneCSGClipping, _>(|c| c.write().write(scene, root.some_handle()));
}

pub struct CSGClippingRenderer {
  expressions: AbstractReadonlyStorageBuffer<[u32]>,
  scene_csg: LockReadGuardHolder<UniformBufferCollectionRaw<u32, Vec4<u32>>>,
  fill_face: bool,
}

impl CSGClippingRenderer {
  /// pass scene because not scene has clipping
  pub fn fill_face(&self, scene: EntityHandle<SceneEntity>) -> bool {
    self.fill_face && self.scene_csg.get(&scene.alloc_index()).is_some()
  }

  pub fn use_get_scene_clipping(
    &self,
    scene_id: EntityHandle<SceneEntity>,
    ctx: &mut FrameCtx,
  ) -> (
    Option<Box<dyn RenderComponent>>,
    Option<AtomicImageDowngrade>,
  ) {
    let fill_face_depth = if self.fill_face && self.scene_csg.get(&scene_id.alloc_index()).is_some()
    {
      ctx.scope(|ctx| {
        let (ctx, image) = ctx.use_plain_state_default::<Option<AtomicImageDowngrade>>();

        if let Some(i) = image {
          if i.size() != ctx.frame_size() {
            *image = None;
          }
        }

        let image = image
          .get_or_insert_with(|| AtomicImageDowngrade::new(&ctx.gpu.device, ctx.frame_size(), 2));

        image.clear(
          &ctx.gpu.device,
          &mut ctx.encoder,
          FRONT_FACE_LAYER_IDX,
          0_f32.to_bits(),
        ); // todo reverse
        image.clear(
          &ctx.gpu.device,
          &mut ctx.encoder,
          BACKFACE_LAYER_IDX,
          0_f32.to_bits(),
        ); // todo reverse

        Some(image.clone())
      })
    } else {
      None
    };

    let r = self.scene_csg.get(&scene_id.alloc_index()).map(|root| {
      let clip_id = ClippingRootDirectProvide { root: root.clone() };

      let csg_clip = CSGExpressionClippingComponent {
        fill_face_depth: fill_face_depth.clone(),
        expressions: self.expressions.clone(),
      };

      // todo, reduce boxing
      let compose = RenderArray([
        Box::new(clip_id) as Box<dyn RenderComponent>,
        Box::new(csg_clip) as Box<dyn RenderComponent>,
      ]);

      Box::new(compose) as Box<dyn RenderComponent>
    });

    (r, fill_face_depth)
  }
}

pub fn use_csg_clipping(cx: &mut QueryGPUHookCx) -> Option<CSGClippingRenderer> {
  let expressions = use_csg_device_data(cx);

  let scene_csg = cx.use_uniform_buffers();

  cx.use_changes::<SceneCSGClipping>()
    .filter_map_changes(|v| {
      let id = v?.index();
      Vec4::new(id, 0, 0, 0).into()
    })
    .update_uniforms(&scene_csg, 0, cx.gpu);

  cx.when_render(|| CSGClippingRenderer {
    expressions: expressions.unwrap(),
    scene_csg: scene_csg.make_read_holder(),
    fill_face: true,
  })
}

struct ClippingRootDirectProvide {
  root: UniformBufferDataView<Vec4<u32>>,
}
impl ShaderHashProvider for ClippingRootDirectProvide {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for ClippingRootDirectProvide {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.root);
  }
}
impl GraphicsShaderProvider for ClippingRootDirectProvide {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, b| {
      let root = self.root.bind_shader(b).load().x();
      builder.register::<SceneModelClippingId>(root);
    })
  }
}

struct CSGExpressionClippingComponent {
  expressions: AbstractReadonlyStorageBuffer<[u32]>,
  fill_face_depth: Option<AtomicImageDowngrade>,
}

impl ShaderHashProvider for CSGExpressionClippingComponent {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.fill_face_depth.is_some().hash(hasher);
  }
}

impl ShaderPassBuilder for CSGExpressionClippingComponent {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.expressions);
    if let Some(fill_face_depth) = &self.fill_face_depth {
      fill_face_depth.bind(&mut ctx.binding);
    }
  }
}

const BACKFACE_LAYER_IDX: u32 = 0;
const FRONT_FACE_LAYER_IDX: u32 = 1;

only_fragment!(SceneModelClippingId, u32);

impl GraphicsShaderProvider for CSGExpressionClippingComponent {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      if self.fill_face_depth.is_some() {
        builder.primitive_state.cull_mode = None;
      }
    });

    builder.fragment(|builder, b| {
      let expressions = AbstractShaderBindingSource::bind_shader(&self.expressions, b);
      let root = builder.query::<SceneModelClippingId>();
      let position =
        builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      let cam_position = builder.query::<CameraWorldPositionHP>();

      // todo, support high precision rendering
      let cam_position = cam_position.expand().f1;
      let world_position = position + cam_position;

      if let Some(fill_face_depth) = &self.fill_face_depth {
        let extra_depth = fill_face_depth.build(b);
        let is_front = builder.query::<FragmentFrontFacing>();
        let frag_position = builder.query::<FragmentPosition>();
        let frag_position_write = frag_position.xy().into_u32();
        if_by(is_front, || {
          // todo, reverse z config
          extra_depth.atomic_max(
            frag_position_write,
            val(FRONT_FACE_LAYER_IDX),
            frag_position.z().bitcast::<u32>(),
          );
        })
        .else_by(|| {
          extra_depth.atomic_max(
            frag_position_write,
            val(BACKFACE_LAYER_IDX),
            frag_position.z().bitcast::<u32>(),
          );
        });
      }

      let distance = eval_distance(world_position, root, &expressions);
      if_by(distance.less_than(val(0.)), || {
        builder.discard();
      });
    })
  }
}

pub fn create_clip_pick_filter(
) -> impl Fn(&MeshBufferHitPoint<f64>, EntityHandle<SceneModelEntity>) -> bool {
  let csg_eval = CSGxSDFxEvaluator::default();
  let sm_ref_scene = read_global_db_foreign_key::<SceneModelBelongsToScene>();
  let scene_csg_root = read_global_db_foreign_key::<SceneCSGClipping>();

  move |v, id| {
    let scene_id = sm_ref_scene.get(id).unwrap();
    if let Some(scene_csg_root) = scene_csg_root.get(scene_id) {
      let position = v.hit.position.into_f32();
      if let Some(v) = csg_eval.eval_distance(position, scene_csg_root) {
        v >= 0.
      } else {
        true
      }
    } else {
      true
    }
  }
}

pub enum CSGxClipFillType<'a> {
  Forward {
    scene_result: &'a RenderTargetView,
    forward_lighting: &'a dyn RenderComponent,
  },
  Defer(&'a FrameGeneralMaterialBuffer),
}

impl CSGClippingRenderer {
  pub fn draw_csg_surface(
    &self,
    frame_ctx: &mut FrameCtx,
    g_buffer_target: &FrameGeometryBuffer,
    fill_depth_info: AtomicImageDowngrade,
    target: CSGxClipFillType,
    camera_gpu: &CameraGPU,
    scene: EntityHandle<SceneEntity>,
    reverse_z: bool,
  ) {
    if !self.fill_face {
      return;
    }

    let root = self
      .scene_csg
      .get(&scene.alloc_index())
      .map(|root| ClippingRootDirectProvide { root: root.clone() });

    if root.is_none() {
      return;
    }
    let root = &root.unwrap() as &dyn RenderComponent;

    // first, fill the face, write the depth buffer.
    let draw = RayMarchingCsgExpression {
      expressions: self.expressions.clone(),
      camera_gpu: camera_gpu.clone(),
      fill_depth_info,
      clip_normal: g_buffer_target
        .normal
        .expect_standalone_common_texture_view_for_binding()
        .clone()
        .try_into()
        .unwrap(),
      clip_depth: g_buffer_target
        .depth
        .expect_standalone_common_texture_view_for_binding()
        .clone()
        .try_into()
        .unwrap(),
      reverse_depth: reverse_z,
    };

    let mut draw = RenderArray([root, &draw]).draw_quad();

    let fill_depth = depth_attachment().request(frame_ctx);

    pass("csg fill surface")
      .with_depth(&fill_depth, clear_and_store(0.), load_and_store())
      .render_ctx(frame_ctx)
      .by(&mut draw);

    // then, copy filled depth to targets, and compute normal in image space only for filled surface.
    // and write other necessary info or directly compute the result in targets

    match target {
      CSGxClipFillType::Forward {
        forward_lighting,
        scene_result,
      } => {
        let mut pass = pass("csg fill surface direct forward shading");

        let color_writer =
          DefaultDisplayWriter::extend_pass_desc(&mut pass, scene_result, load_and_store());
        let g_buffer_base_writer = g_buffer_target.extend_pass_desc_for_subsequent_draw(&mut pass);

        let draw = ForwardCsgSurfaceDraw {
          filled_depth: fill_depth
            .expect_standalone_common_texture_view_for_binding()
            .clone()
            .try_into()
            .unwrap(),
        };

        let mut draw = RenderArray([
          &color_writer as &dyn RenderComponent,
          &g_buffer_base_writer as &dyn RenderComponent,
          // forward_lighting,
          &draw,
        ])
        .draw_quad();

        pass.render_ctx(frame_ctx).by(&mut draw);
      }
      CSGxClipFillType::Defer(_frame_general_material_buffer) => todo!(),
    }
  }
}

struct ForwardCsgSurfaceDraw {
  filled_depth: GPU2DDepthTextureView,
}

impl ShaderHashProvider for ForwardCsgSurfaceDraw {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for ForwardCsgSurfaceDraw {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.filled_depth.bind_pass(&mut ctx.binding);
  }
}

impl GraphicsShaderProvider for ForwardCsgSurfaceDraw {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      // todo, compute normal

      let frag_position = builder.query::<FragmentPosition>().xy().into_u32();
      let depth = self.filled_depth.bind_shader(binding);
      let depth = depth.load_texel(frag_position, val(0));

      // todo write material data
      builder.register::<DefaultDisplay>(val(Vec4::one()));
      builder.register::<LogicalRenderEntityId>(val(u32::MAX)); // todo, use max-1 to distinguish the background,but need fix gpu picking
      builder.register::<FragmentRenderNormal>(val(Vec3::new(1.0, 0., 0.)));

      // override quad draw config
      let depth_stencil = builder.depth_stencil.as_mut().unwrap();
      depth_stencil.depth_compare = CompareFunction::Greater; // todo reverse z
      builder.depth_stencil.as_mut().unwrap().depth_write_enabled = true;

      builder.register::<FragmentDepthOutput>(depth);
    })
  }
}

struct RayMarchingCsgExpression {
  camera_gpu: CameraGPU,
  expressions: AbstractReadonlyStorageBuffer<[u32]>,
  clip_depth: GPU2DDepthTextureView,
  clip_normal: GPU2DTextureView,
  fill_depth_info: AtomicImageDowngrade,
  reverse_depth: bool,
}

impl ShaderHashProvider for RayMarchingCsgExpression {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
  }
}

impl ShaderPassBuilder for RayMarchingCsgExpression {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera_gpu.setup_pass(ctx);
    ctx.binding.bind(&self.expressions);
    self.clip_depth.bind_pass(&mut ctx.binding);
    self.clip_normal.bind_pass(&mut ctx.binding);
    self.fill_depth_info.bind(&mut ctx.binding);
  }
}

impl GraphicsShaderProvider for RayMarchingCsgExpression {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera_gpu.inject_uniforms(builder);
    builder.fragment(|builder, binding| {
      let expressions = AbstractShaderBindingSource::bind_shader(&self.expressions, binding);
      let clip_depth = self.clip_depth.bind_shader(binding);
      let clip_normal = self.clip_normal.bind_shader(binding);
      let fill_depth_info = self.fill_depth_info.build(binding);
      let frag_position = builder.query::<FragmentPosition>().xy().into_u32();
      let uv = builder.query::<FragmentUv>();
      let camera_position_world = builder.query::<CameraWorldPositionHP>().expand().f1;
      let ndc_to_render = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
      let render_to_ndc = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
      let root = builder.query::<SceneModelClippingId>();

      let background_depth = if self.reverse_depth { val(0.) } else { val(1.) };
      // let near_depth = if self.reverse_depth { val(1.) } else { val(0.) };

      let clip_depth = clip_depth.load_texel(frag_position, val(0));
      let clip_normal = clip_normal.load_texel(frag_position, val(0)).xyz();
      // todo, use common load
      let back_depth = fill_depth_info
        .atomic_load(frag_position, val(BACKFACE_LAYER_IDX))
        .bitcast::<f32>();
      let front_depth = fill_depth_info
        .atomic_load(frag_position, val(FRONT_FACE_LAYER_IDX))
        .bitcast::<f32>();

      let has_clip = clip_depth.not_equals(background_depth);
      let has_back = back_depth.not_equals(background_depth);
      let has_front = front_depth.not_equals(background_depth);

      let should_check = has_clip.or(has_back).or(has_front);
      let not_require = has_clip.and(clip_depth.equals(front_depth));
      let should_check = should_check.and(not_require.not());

      let output_depth = background_depth.make_local_var();

      if_by(should_check, || {
        let start = compute_start_point_fn(uv, back_depth, camera_position_world, ndc_to_render);
        let dir = (camera_position_world - start).normalize();
        let (back_to_front_marched_depth, back_to_front_intersected) = ray_marching(
          &expressions,
          root,
          start,
          dir,
          camera_position_world,
          render_to_ndc,
        );

        let start = compute_start_point_fn(uv, front_depth, camera_position_world, ndc_to_render);
        let dir = (start - camera_position_world).normalize();
        let (front_to_back_marched_depth, front_to_back_intersected) = ray_marching(
          &expressions,
          root,
          start,
          dir,
          camera_position_world,
          render_to_ndc,
        );

        let fill_depth = clip_depth.make_local_var();
        if_by(has_clip, || {
          let clip_point =
            compute_start_point_fn(uv, clip_depth, camera_position_world, ndc_to_render);
          let dir = clip_point - camera_position_world;
          let is_clip_surface_back_face = clip_normal.dot(dir).greater_than(val(0.));

          if_by(is_clip_surface_back_face, || {
            if_by(
              front_to_back_intersected
                .and(front_to_back_marched_depth.greater_than(fill_depth.load())),
              || {
                fill_depth.store(front_to_back_marched_depth);
              },
            );
            if_by(
              back_to_front_intersected
                .and(back_to_front_marched_depth.greater_than(fill_depth.load())),
              || {
                fill_depth.store(back_to_front_marched_depth);
              },
            );
          });
        })
        .else_by(|| {
          if_by(
            front_to_back_intersected
              .and(front_to_back_marched_depth.greater_than(fill_depth.load()))
              .and(front_to_back_marched_depth.greater_than(back_depth)),
            || {
              fill_depth.store(front_to_back_marched_depth);
            },
          );
          if_by(
            back_to_front_intersected
              .and(back_to_front_marched_depth.greater_than(fill_depth.load()))
              .and(back_to_front_marched_depth.less_than(front_depth)),
            || {
              fill_depth.store(back_to_front_marched_depth);
            },
          );
        });
        output_depth.store(fill_depth.load());
      });

      // override quad draw config
      builder.depth_stencil.as_mut().unwrap().depth_write_enabled = true;
      builder.register::<FragmentDepthOutput>(output_depth.load());
    });
  }
}

#[shader_fn]
fn compute_start_point(
  uv: Node<Vec2<f32>>,
  depth: Node<f32>,
  camera_position_world: Node<Vec3<f32>>,
  ndc_to_render: Node<Mat4<f32>>,
) -> Node<Vec3<f32>> {
  let uv = uv * val(Vec2::splat(2.)) - val(Vec2::splat(1.));
  let uv = uv * val(Vec2::new(1., -1.));
  let ndc = (uv, depth, val(1.)).into();
  let render = ndc_to_render * ndc;
  let render = render.xyz() / render.w().splat();
  render + camera_position_world
}

/// return (final position, if intersected)
fn ray_marching(
  expressions: &ShaderReadonlyPtrOf<[u32]>,
  root: Node<u32>,
  start: Node<Vec3<f32>>,
  ray_dir: Node<Vec3<f32>>,
  camera_position_world: Node<Vec3<f32>>,
  render_to_ndc: Node<Mat4<f32>>,
) -> (Node<f32>, Node<bool>) {
  let surface_point = start.make_local_var();
  let intersected = val(true).make_local_var();

  // raymarching
  let eval_count = val(0_u32).make_local_var();
  loop_by(|lcx| {
    let distance = eval_distance(surface_point.load(), root, expressions);

    // not in clip part
    if_by(distance.greater_equal_than(val(0.)), || {
      intersected.store(false);
      lcx.do_break();
    });

    // skip the none clip surface or close enough surface
    if_by(distance.greater_than(-val(0.01)), || {
      lcx.do_break();
    });

    let eval_c = eval_count.load();
    if_by(eval_c.greater_equal_than(val(20)), || {
      intersected.store(false);
      lcx.do_break();
    });

    // because we are marching from clipped(<0) to none clipped(>0), so this is negative
    surface_point.store(surface_point.load() - ray_dir * distance);
    eval_count.store(eval_c + val(1));
  });

  let surface_point = surface_point.load();

  let surface_point_in_render = surface_point - camera_position_world;
  let surface_point_in_ndc = render_to_ndc * vec4_node((surface_point_in_render, val(1.)));
  let z = surface_point_in_ndc.z() / surface_point_in_ndc.w();

  (z, intersected.load())
}
