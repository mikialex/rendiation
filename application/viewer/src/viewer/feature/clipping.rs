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

  let root = w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Min))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
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

  pub fn get_stencil_load_op(&self) -> Option<Operations<u32>> {
    if self.fill_face {
      Some(clear_and_store(CSG_CLIPPING_FILL_FACE_STENCIL_CLEAR))
    } else {
      None
    }
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

        // todo, support layer clear
        // image.clear(&ctx.gpu.device, &mut ctx.encoder, value);

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

pub const CSG_CLIPPING_FILL_FACE_STENCIL_CLEAR: u32 = 0;
pub const CSG_CLIPPING_FILL_FACE_STENCIL_BACKFACE_REF: u32 = 1;

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
        let frag_position_uv = frag_position.xy() * val(Vec2::splat(0.5)) + val(Vec2::splat(0.5));
        let frag_position_write = frag_position_uv * extra_depth.size().into_f32();
        let frag_position_write = frag_position_write.into_u32();
        if_by(is_front, || {
          // todo, reverse z config
          extra_depth.atomic_min(
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
    assert!(self.fill_face);

    // let root = self
    //   .scene_csg
    //   .get(&scene.alloc_index())
    //   .map(|root| ClippingRootDirectProvide { root: root.clone() });

    // if root.is_none() {
    //   return;
    // }
    // let root = &root.unwrap() as &dyn RenderComponent;

    // // first, fill the face, write the depth buffer.
    // let draw = RayMarchingCsgExpression {
    //   expressions: self.expressions.clone(),
    //   camera_gpu: camera_gpu.clone(),
    //   reverse_depth: reverse_z,
    // };

    // let mut draw = RenderArray([root, &draw]).draw_quad();

    // pass("csg fill surface")
    //   .with_depth(
    //     &g_buffer_target.depth,
    //     clear_and_store(0.),
    //     load_and_store(),
    //   )
    //   .render_ctx(frame_ctx)
    //   .by(&mut draw);

    // match target {
    //   CSGxClipFillType::Forward {
    //     forward_lighting,
    //     scene_result,
    //   } => {
    //     let mut pass = pass("csg fill surface direct forward shading");

    //     let color_writer =
    //       DefaultDisplayWriter::extend_pass_desc(&mut pass, scene_result, load_and_store());
    //     let g_buffer_base_writer = g_buffer_target.extend_pass_desc_for_subsequent_draw(&mut pass);

    //     let depth = g_buffer_target
    //       .depth
    //       .expect_standalone_common_texture_view_for_binding()
    //       .clone();

    //     let draw = ForwardCsgSurfaceDraw {
    //       depth: depth.try_into().unwrap(),
    //     };

    //     let mut draw = RenderArray([
    //       &color_writer as &dyn RenderComponent,
    //       &g_buffer_base_writer as &dyn RenderComponent,
    //       // forward_lighting,
    //       &draw,
    //     ])
    //     .draw_quad();

    //     pass.render_ctx(frame_ctx).by(&mut draw);
    //   }
    //   CSGxClipFillType::Defer(_frame_general_material_buffer) => todo!(),
    // }

    // then, compute normal in image space only for filled surface.
  }
}

struct ForwardCsgSurfaceDraw {
  depth: GPU2DDepthTextureView,
}

impl ShaderHashProvider for ForwardCsgSurfaceDraw {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for ForwardCsgSurfaceDraw {
  fn setup_pass(&self, _ctx: &mut GPURenderPassCtx) {}
}

impl GraphicsShaderProvider for ForwardCsgSurfaceDraw {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let depth_stencil = builder.depth_stencil.as_mut().unwrap();

      // only execute on fill area
      depth_stencil.stencil.front.compare = CompareFunction::Equal;

      // todo, bind depth and compute normal
      // let depth = self.depth.b

      // let uv = builder.query::<FragmentUv>();

      // todo write material data
      builder.register::<DefaultDisplay>(val(Vec4::one()));
      builder.register::<LogicalRenderEntityId>(val(u32::MAX));
      builder.register::<FragmentRenderNormal>(val(Vec3::new(1.0, 0., 0.)));
    })
  }
}

struct RayMarchingCsgExpression {
  expressions: AbstractReadonlyStorageBuffer<[u32]>,
  camera_gpu: CameraGPU,
  reverse_depth: bool,
}

impl ShaderHashProvider for RayMarchingCsgExpression {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
  }
}

impl GraphicsShaderProvider for RayMarchingCsgExpression {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera_gpu.inject_uniforms(builder);
    builder.fragment(|builder, binding| {
      let depth_stencil = builder.depth_stencil.as_mut().unwrap();

      // only write on fill area
      // todo, only execute on fill area, (we can not express early stencil test)
      // to do so, we need bind stencil only part of the depth buffer and write to another temp texture.
      depth_stencil.stencil.front.compare = CompareFunction::Equal;

      // override the quad draw setting
      depth_stencil.depth_compare = if self.reverse_depth {
        CompareFunction::Greater
      } else {
        CompareFunction::Less
      };
      depth_stencil.depth_write_enabled = true;

      let uv = builder.query::<FragmentUv>();
      let expressions = AbstractShaderBindingSource::bind_shader(&self.expressions, binding);
      let root = builder.query::<SceneModelClippingId>();

      let near = if self.reverse_depth { val(1.) } else { val(0.) };

      let start_point_in_ndc: Node<Vec3<f32>> = (uv * val(2.) - val(Vec2::splat(1.)), near).into();
      let mat = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
      let start_point_in_render_space = mat * vec4_node((start_point_in_ndc, val(1.)));
      let start_point_in_render_space =
        start_point_in_render_space.xyz() / start_point_in_render_space.w().splat();
      let camera_position = builder.query::<CameraWorldPositionHP>().expand().f1;
      // todo high precision support
      let start_point_in_world = start_point_in_render_space + camera_position;
      let dir = start_point_in_render_space.normalize();

      let surface_point = start_point_in_world.make_local_var();
      let no_intersect = val(false).make_local_var();

      // raymarching
      let eval_count = val(0_u32).make_local_var();
      loop_by(|lcx| {
        let distance = eval_distance(start_point_in_world, root, &expressions);

        surface_point.store(surface_point.load() + dir * distance);

        if_by(distance.less_than(val(0.1)), || {
          lcx.do_break();
        });
        let eval_c = eval_count.load();
        if_by(eval_c.greater_equal_than(val(5)), || {
          no_intersect.store(true);
          lcx.do_break();
        });

        eval_count.store(eval_c + val(1));
      });

      if_by(no_intersect.load(), || {
        builder.discard();
      });

      let surface_point = surface_point.load();
      let surface_point_in_render = surface_point - camera_position;
      let mat = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
      let surface_point_in_ndc = mat * vec4_node((surface_point_in_render, val(1.)));
      let z = surface_point_in_ndc.z() / surface_point_in_ndc.w();
      builder.register::<FragmentDepthOutput>(z);
    });
  }
}

impl ShaderPassBuilder for RayMarchingCsgExpression {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_stencil_reference(CSG_CLIPPING_FILL_FACE_STENCIL_BACKFACE_REF);
    self.camera_gpu.setup_pass(ctx);
    ctx.binding.bind(&self.expressions);
  }
}

// if self.fill_face {
//   let ray_dir = (world_position - cam_position).normalize();
//   let is_front = builder.query::<FragmentFrontFacing>();
//   let ray_dir = is_front.select(ray_dir, -ray_dir);

//   let surface_point = world_position.make_local_var();
//   let no_intersect = val(false).make_local_var();

//   // raymarching
//   let eval_count = val(0_u32).make_local_var();
//   loop_by(|lcx| {
//     let distance = eval_distance(surface_point.load(), root, &expressions);

//     // skip the none clip surface and close enough surface
//     if_by(distance.greater_than(-val(0.01)), || {
//       lcx.do_break();
//     });

//     let eval_c = eval_count.load();
//     if_by(eval_c.greater_equal_than(val(20)), || {
//       no_intersect.store(true);
//       lcx.do_break();
//     });

//     surface_point.store(surface_point.load() - ray_dir * distance);
//     eval_count.store(eval_c + val(1));
//   });

//   if_by(no_intersect.load(), || {
//     builder.discard();
//   });

//   let surface_point = surface_point.load();
//   let surface_point_in_render = surface_point - cam_position;
//   let mat = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
//   let surface_point_in_ndc = mat * vec4_node((surface_point_in_render, val(1.)));
//   let z = surface_point_in_ndc.z() / surface_point_in_ndc.w();
//   builder.register::<FragmentDepthOutput>(z);
// }
