use rendiation_infinity_primitive::ShaderPlane;

use crate::*;

pub const MAX_CLIPPING_PLANE_SUPPORT_IN_CLIPPING_SET: usize = 8;

pub fn register_clipping_data_model() {
  global_database()
    .declare_entity::<CSGExpressionNodeEntity>()
    .declare_component::<CSGExpressionNodeContent>()
    .declare_foreign_key::<CSGExpressionLeftChild>()
    .declare_foreign_key::<CSGExpressionRightChild>();

  global_entity_of::<SceneEntity>().declare_foreign_key::<SceneCSGClipping>();
}

declare_entity!(CSGExpressionNodeEntity);
declare_component!(
  CSGExpressionNodeContent,
  CSGExpressionNodeEntity,
  Option<CSGExpressionNode>
);
declare_foreign_key!(
  CSGExpressionLeftChild,
  CSGExpressionNodeEntity,
  CSGExpressionNodeEntity
);
declare_foreign_key!(
  CSGExpressionRightChild,
  CSGExpressionNodeEntity,
  CSGExpressionNodeEntity
);

declare_foreign_key!(SceneCSGClipping, SceneEntity, CSGExpressionNodeEntity);

#[repr(C)]
#[derive(Clone, Debug, Facet, Serialize, Deserialize, PartialEq)]
pub enum CSGExpressionNode {
  Plane(Plane),
  And,
  Or,
}

pub fn load_testing_clipping_data() -> EntityHandle<CSGExpressionNodeEntity> {
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
  let p2 = write_plane(&mut w, Vec3::new(0., 1., 0.), 0.);

  w.new_entity(|w| {
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::And))
      .write::<CSGExpressionLeftChild>(&p1.some_handle())
      .write::<CSGExpressionRightChild>(&p2.some_handle())
  })
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
        Box::new(clip_id) as Box<dyn RenderComponent>,
        Box::new(csg_clip),
      ]);

      Box::new(compose) as Box<dyn RenderComponent>
    })
  }
}

pub fn use_csg_clipping(cx: &mut QueryGPUHookCx) -> Option<CSGClippingRenderer> {
  let (cx, storages) = cx.use_storage_buffer::<u32>("csg expression pool", 128, u32::MAX);

  const EXPR_U32_PAYLOAD_WIDTH: usize = 5;
  const EXPR_BYTE_PAYLOAD_WIDTH: usize = EXPR_U32_PAYLOAD_WIDTH * 4;
  const EXPR_U32_WIDTH: usize = 7;
  const EXPR_BYTE_WIDTH: usize = EXPR_U32_WIDTH * 4;

  cx.use_changes::<CSGExpressionNodeContent>()
    .map_changes(|c| match c {
      Some(c) => match c {
        CSGExpressionNode::Plane(hyper_plane) => [
          3,
          hyper_plane.normal.x.to_bits(),
          hyper_plane.normal.y.to_bits(),
          hyper_plane.normal.z.to_bits(),
          hyper_plane.constant.to_bits(),
        ],
        CSGExpressionNode::And => [1, 0, 0, 0, 0],
        CSGExpressionNode::Or => [2, 0, 0, 0, 0],
      },
      None => [0; 5],
    })
    .update_gpu_buffer_array_raw(cx, storages.collector.as_mut(), 0, EXPR_BYTE_WIDTH);

  cx.use_changes::<CSGExpressionLeftChild>()
    .map_changes(|c| c.map(|v| v.index()).unwrap_or(u32::MAX))
    .update_gpu_buffer_array_raw(
      cx,
      storages.collector.as_mut(),
      EXPR_BYTE_PAYLOAD_WIDTH,
      EXPR_BYTE_WIDTH,
    );

  cx.use_changes::<CSGExpressionRightChild>()
    .map_changes(|c| c.map(|v| v.index()).unwrap_or(u32::MAX))
    .update_gpu_buffer_array_raw(
      cx,
      storages.collector.as_mut(),
      EXPR_BYTE_PAYLOAD_WIDTH + 4,
      EXPR_BYTE_WIDTH,
    );

  storages.use_max_item_count_by_db_entity::<CSGExpressionNodeEntity>(cx);
  storages.use_update(cx);

  let scene_csg = cx.use_uniform_buffers();

  cx.use_changes::<SceneCSGClipping>()
    .map_changes(|v| {
      let id = v.map(|v| v.index()).unwrap_or(u32::MAX);
      Vec4::new(id, 0, 0, 0)
    })
    .update_uniforms(&scene_csg, 0, cx.gpu);

  cx.when_render(|| CSGClippingRenderer {
    expressions: storages.get_gpu_buffer(),
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
  // todo, currently we do clipping at the end, this is not optimal
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, b| {
      let expressions = AbstractShaderBindingSource::bind_shader(&self.expressions, b);
      if let Some(root) = builder.try_query::<SceneModelClippingId>() {
        if let Some(position) = builder.try_query::<FragmentRenderPosition>() {
          if let Some(cam_position) = builder.try_query::<CameraWorldPositionHP>() {
            // todo, support high precision rendering
            let world_position = position + cam_position.expand().f1;
            let should_clip = eval_clipping(world_position, root, &expressions);
            if_by(should_clip, || {
              builder.discard();
            });
          }
        }
      }
    })
  }
}

// todo, support early exit
enum CSGExpressionNodeDeviceVariant {
  Plane(Node<ShaderPlane>),
  InputAnd(Node<u32>, Node<u32>),
  ExecuteAnd,
  InputOr(Node<u32>, Node<u32>),
  ExecuteOr,
}

impl CSGExpressionNodeDeviceVariant {
  pub fn into_node(self) -> CSGExpressionNodeDevice {
    todo!()
  }
}

struct CSGExpressionNodeDevice;

impl CSGExpressionNodeDevice {
  pub fn match_by(&self, f: impl FnOnce(CSGExpressionNodeDeviceVariant)) {
    //
  }
}

struct TreeTraverseStack {
  result_stack: ShaderPtrOf<[bool]>,
  last_result_index: ShaderPtrOf<u32>,
  expr_stack: ShaderPtrOf<[u32]>, // each expr is 5 u32.
  last_expr_index: ShaderPtrOf<u32>,
}

impl Default for TreeTraverseStack {
  fn default() -> Self {
    todo!();
  }
}

impl TreeTraverseStack {
  pub fn push(&self, idx: Node<u32>) {
    //
  }

  pub fn push_raw(&self, action: CSGExpressionNodeDevice) {
    //
  }

  pub fn push_value(&self, item: Node<bool>) {
    let idx = self.last_result_index.load();
    self.last_result_index.store(idx + val(1));
    self.result_stack.index(idx).store(item)
  }

  pub fn pop_value(&self) -> Node<bool> {
    let idx = self.last_result_index.load();
    self.last_result_index.store(idx - val(1));
    self.result_stack.index(idx).load()
  }

  pub fn pop(&self) -> (Node<bool>, CSGExpressionNodeDevice) {
    // let idx = self.last_expr_index.load();
    // let valid = idx.not_equals(val(0));
    // let clamped_idx = valid.select(idx, val(0));
    // self.last_result_index.store(idx - val(5));
    // let expr = self.read_expr(clamped_idx);
    // (valid, todo!())

    todo!()
  }

  fn read_expr(&self, raw_idx: Node<u32>) -> CSGExpressionNodeDevice {
    todo!()
  }
}

fn eval_clipping(
  world_position: Node<Vec3<f32>>,
  root: Node<u32>,
  expression_nodes: &ShaderReadonlyPtrOf<[u32]>,
) -> Node<bool> {
  let stack = TreeTraverseStack::default();
  stack.push(root);

  loop_by(|cx| {
    let (has_next, next_node) = stack.pop();
    if_by(has_next.not(), || cx.do_break());

    next_node.match_by(|v| match v {
      CSGExpressionNodeDeviceVariant::Plane(node) => {
        stack.push_value(eval_plane_clipping_fn(world_position, node));
      }
      CSGExpressionNodeDeviceVariant::InputAnd(left, right) => {
        stack.push_raw(CSGExpressionNodeDeviceVariant::ExecuteAnd.into_node());
        stack.push(left);
        stack.push(right);
      }
      CSGExpressionNodeDeviceVariant::InputOr(left, right) => {
        stack.push_raw(CSGExpressionNodeDeviceVariant::ExecuteOr.into_node());
        stack.push(left);
        stack.push(right);
      }
      CSGExpressionNodeDeviceVariant::ExecuteAnd => {
        let and = stack.pop_value().and(stack.pop_value());
        stack.push_value(and);
      }
      CSGExpressionNodeDeviceVariant::ExecuteOr => {
        let or = stack.pop_value().or(stack.pop_value());
        stack.push_value(or);
      }
    });
  });

  stack.pop_value()
}

#[shader_fn]
fn eval_plane_clipping(world_position: Node<Vec3<f32>>, plane: Node<ShaderPlane>) -> Node<bool> {
  todo!()
}
