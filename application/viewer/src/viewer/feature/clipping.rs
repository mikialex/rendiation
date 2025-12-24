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
    w.write::<CSGExpressionNodeContent>(&Some(CSGExpressionNode::Or))
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

const EXPR_U32_PAYLOAD_WIDTH: usize = 5;
const EXPR_BYTE_PAYLOAD_WIDTH: usize = EXPR_U32_PAYLOAD_WIDTH * 4;
const EXPR_U32_WIDTH: usize = 7;
const EXPR_BYTE_WIDTH: usize = EXPR_U32_WIDTH * 4;

const PLANE_TAG: u32 = 3;
const AND_TAG: u32 = 1;
const OR_TAG: u32 = 2;

pub fn use_csg_clipping(cx: &mut QueryGPUHookCx) -> Option<CSGClippingRenderer> {
  let (cx, storages) = cx.use_storage_buffer::<u32>("csg expression pool", 128, u32::MAX);

  cx.use_changes::<CSGExpressionNodeContent>()
    .map_changes(|c| match c {
      Some(c) => match c {
        CSGExpressionNode::Plane(hyper_plane) => [
          PLANE_TAG,
          hyper_plane.normal.x.to_bits(),
          hyper_plane.normal.y.to_bits(),
          hyper_plane.normal.z.to_bits(),
          hyper_plane.constant.to_bits(),
        ],
        CSGExpressionNode::And => [AND_TAG, 0, 0, 0, 0],
        CSGExpressionNode::Or => [OR_TAG, 0, 0, 0, 0],
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
    .filter_map_changes(|v| {
      let id = v?.index();
      Vec4::new(id, 0, 0, 0).into()
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
      let root = builder.query::<SceneModelClippingId>();
      let position =
        builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      let cam_position = builder.query::<CameraWorldPositionHP>();

      // todo, support high precision rendering
      let world_position = position + cam_position.expand().f1;
      let should_clip = eval_clipping(world_position, root, &expressions);
      if_by(should_clip, || {
        builder.discard();
      });
    })
  }
}

pub const MAX_CSG_EVAL_STACK_SIZE: usize = 32;

struct TreeTraverseStack {
  result_stack: ShaderPtrOf<[bool; MAX_CSG_EVAL_STACK_SIZE]>,
  result_len: ShaderPtrOf<u32>,
  expr_stack: ShaderPtrOf<[u32; MAX_CSG_EVAL_STACK_SIZE]>, // each expr is 5 u32.
  expr_len: ShaderPtrOf<u32>,
}

impl Default for TreeTraverseStack {
  fn default() -> Self {
    let result_stack = zeroed_val::<[bool; MAX_CSG_EVAL_STACK_SIZE]>();
    let expr_stack = zeroed_val::<[u32; MAX_CSG_EVAL_STACK_SIZE]>();
    Self {
      result_stack: result_stack.make_local_var(),
      result_len: val(0_u32).make_local_var(),
      expr_stack: expr_stack.make_local_var(),
      expr_len: val(0_u32).make_local_var(),
    }
  }
}

const OR_ACTION_TAG: u32 = u32::MAX - 1;
const AND_ACTION_TAG: u32 = u32::MAX - 2;

impl TreeTraverseStack {
  pub fn push(&self, action: CSGExpressionNodeDeviceAction) {
    let node_or_tag = match action {
      CSGExpressionNodeDeviceAction::Input(node) => node,
      CSGExpressionNodeDeviceAction::AndAction => val(AND_ACTION_TAG),
      CSGExpressionNodeDeviceAction::OrAction => val(OR_ACTION_TAG),
    };
    let idx = self.expr_len.load();
    self.expr_len.store(idx + val(1));
    self.expr_stack.index(idx).store(node_or_tag);
  }

  pub fn push_result(&self, item: Node<bool>) {
    let idx = self.result_len.load();
    self.result_len.store(idx + val(1));
    self.result_stack.index(idx).store(item)
  }

  pub fn pop_result(&self) -> Node<bool> {
    let idx = self.result_len.load();
    let read_idx = idx - val(1);
    self.result_len.store(read_idx);
    self.result_stack.index(read_idx).load()
  }

  pub fn pop(
    &self,
    expr_pool: &ShaderReadonlyPtrOf<[u32]>,
  ) -> (Node<bool>, CSGExpressionNodeDevice) {
    let idx = self.expr_len.load();

    let valid = idx.not_equals(val(0));
    let clamped_idx = valid.select(idx - val(1), val(0));
    let expr = self.read_expr(clamped_idx, expr_pool);

    if_by(valid, || self.expr_len.store(clamped_idx));

    (valid, expr)
  }

  fn read_expr(
    &self,
    idx: Node<u32>,
    expr_pool: &ShaderReadonlyPtrOf<[u32]>,
  ) -> CSGExpressionNodeDevice {
    let idx_or_tag = self.expr_stack.index(idx).load();

    CSGExpressionNodeDevice {
      idx_or_tag,
      expr_pool: expr_pool.clone(),
    }
  }
}

enum CSGExpressionNodeDeviceVariant {
  Plane(ENode<ShaderPlane>),
  InputAnd(Node<u32>, Node<u32>),
  ExecuteAnd,
  InputOr(Node<u32>, Node<u32>),
  ExecuteOr,
}

enum CSGExpressionNodeDeviceAction {
  Input(Node<u32>),
  AndAction,
  OrAction,
}

struct CSGExpressionNodeDevice {
  expr_pool: ShaderReadonlyPtrOf<[u32]>,
  idx_or_tag: Node<u32>,
}

impl CSGExpressionNodeDevice {
  pub fn match_by(&self, f: impl Fn(CSGExpressionNodeDeviceVariant)) {
    switch_by(self.idx_or_tag)
      .case(OR_ACTION_TAG, || {
        f(CSGExpressionNodeDeviceVariant::ExecuteOr)
      })
      .case(AND_ACTION_TAG, || {
        f(CSGExpressionNodeDeviceVariant::ExecuteAnd)
      })
      .end_with_default(|| {
        let pool_offset = self.idx_or_tag * val(EXPR_U32_WIDTH as u32);
        let tag = self.expr_pool.index(pool_offset).load();
        switch_by(tag)
          .case(PLANE_TAG, || {
            let offset = pool_offset + val(1);
            let normal = Node::<Vec3<f32>>::load_from_u32_buffer(
              &self.expr_pool,
              offset,
              StructLayoutTarget::Packed,
            );
            let constant = self
              .expr_pool
              .index(offset + val(3))
              .load()
              .bitcast::<f32>();
            let plane = ENode::<ShaderPlane> { normal, constant };
            f(CSGExpressionNodeDeviceVariant::Plane(plane))
          })
          .case(AND_TAG, || {
            let offset = pool_offset + val(EXPR_U32_PAYLOAD_WIDTH as u32);
            let left = self.expr_pool.index(offset).load();
            let right = self.expr_pool.index(offset + val(1)).load();
            f(CSGExpressionNodeDeviceVariant::InputAnd(left, right))
          })
          .case(OR_TAG, || {
            let offset = pool_offset + val(EXPR_U32_PAYLOAD_WIDTH as u32);
            let left = self.expr_pool.index(offset).load();
            let right = self.expr_pool.index(offset + val(1)).load();
            f(CSGExpressionNodeDeviceVariant::InputOr(left, right))
          })
          .end_with_default(|| {
            // unreachable
          });
      });
  }
}

fn eval_clipping(
  world_position: Node<Vec3<f32>>,
  root: Node<u32>,
  expression_nodes: &ShaderReadonlyPtrOf<[u32]>,
) -> Node<bool> {
  let stack = TreeTraverseStack::default();
  stack.push(CSGExpressionNodeDeviceAction::Input(root));

  loop_by(|cx| {
    let (has_next, next_node) = stack.pop(expression_nodes);
    if_by(has_next.not(), || cx.do_break());

    next_node.match_by(|v| match v {
      CSGExpressionNodeDeviceVariant::Plane(node) => {
        stack.push_result(plane_should_clipped(world_position, node));
      }
      CSGExpressionNodeDeviceVariant::InputAnd(left, right) => {
        stack.push(CSGExpressionNodeDeviceAction::AndAction);
        stack.push(CSGExpressionNodeDeviceAction::Input(left));
        stack.push(CSGExpressionNodeDeviceAction::Input(right));
      }
      CSGExpressionNodeDeviceVariant::InputOr(left, right) => {
        stack.push(CSGExpressionNodeDeviceAction::OrAction);
        stack.push(CSGExpressionNodeDeviceAction::Input(left));
        stack.push(CSGExpressionNodeDeviceAction::Input(right));
      }
      CSGExpressionNodeDeviceVariant::ExecuteAnd => {
        let and = stack.pop_result().and(stack.pop_result());
        stack.push_result(and);
      }
      CSGExpressionNodeDeviceVariant::ExecuteOr => {
        let or = stack.pop_result().or(stack.pop_result());
        stack.push_result(or);
      }
    });
  });

  stack.pop_result()
}

fn plane_should_clipped(world_position: Node<Vec3<f32>>, plane: ENode<ShaderPlane>) -> Node<bool> {
  let distance = world_position.dot(plane.normal) + plane.constant;
  distance.less_than(val(0.0))
}
