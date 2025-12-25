use rendiation_csg_sdf_expression::*;

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
}

impl CSGClippingRenderer {
  pub fn get_scene_clipping(
    &self,
    scene_id: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn RenderComponent>> {
    self.scene_csg.get(&scene_id.alloc_index()).map(|root| {
      let clip_id = ClippingRootDirectProvide { root: root.clone() };

      let csg_clip = CSGExpressionClippingComponent {
        expressions: self.expressions.clone(),
      };

      // todo, reduce boxing
      let compose = RenderArray([
        Box::new(csg_clip) as Box<dyn RenderComponent>,
        Box::new(clip_id),
      ]);

      Box::new(compose) as Box<dyn RenderComponent>
    })
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
  })
}

struct ClippingRootDirectProvide {
  root: UniformBufferDataView<Vec4<u32>>,
}
impl ShaderHashProvider for ClippingRootDirectProvide {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for ClippingRootDirectProvide {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.root);
  }
}
impl GraphicsShaderProvider for ClippingRootDirectProvide {
  // todo, currently we do clipping at the end, this is not optimal
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, b| {
      let root = self.root.bind_shader(b).load().x();
      builder.register::<SceneModelClippingId>(root);
    })
  }
}

struct CSGExpressionClippingComponent {
  expressions: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for CSGExpressionClippingComponent {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for CSGExpressionClippingComponent {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.expressions);
  }
}

only_fragment!(SceneModelClippingId, u32);

impl GraphicsShaderProvider for CSGExpressionClippingComponent {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, b| {
      let expressions = AbstractShaderBindingSource::bind_shader(&self.expressions, b);
      let root = builder.query::<SceneModelClippingId>();
      let position =
        builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      let cam_position = builder.query::<CameraWorldPositionHP>();

      // todo, support high precision rendering
      let world_position = position + cam_position.expand().f1;
      let distance = eval_distance(world_position, root, &expressions);
      if_by(distance.less_than(val(0.)), || {
        builder.discard();
      });
    })
  }
}
