use rendiation_shader_library::plane::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

const EXPR_U32_PAYLOAD_WIDTH: usize = 5;
const EXPR_BYTE_PAYLOAD_WIDTH: usize = EXPR_U32_PAYLOAD_WIDTH * 4;
const EXPR_U32_WIDTH: usize = 7;
const EXPR_BYTE_WIDTH: usize = EXPR_U32_WIDTH * 4;

const PLANE_TAG: u32 = 3;
const SPHERE_TAG: u32 = 4;
const MAX_TAG: u32 = 1;
const MIN_TAG: u32 = 2;

pub const MAX_CSG_EVAL_STACK_SIZE: usize = 32;

pub fn use_csg_device_data(
  cx: &mut QueryGPUHookCx,
) -> Option<AbstractReadonlyStorageBuffer<[u32]>> {
  let (cx, storages) = cx.use_storage_buffer::<u32>("csg expression pool", 128, u32::MAX);

  cx.use_changes::<CSGExpressionNodeContent>()
    .map_changes(|c| match c {
      Some(c) => match c {
        CSGExpressionNode::Plane(plane) => [
          PLANE_TAG,
          plane.normal.x.to_bits(),
          plane.normal.y.to_bits(),
          plane.normal.z.to_bits(),
          plane.constant.to_bits(),
        ],
        CSGExpressionNode::Sphere(sphere) => [
          SPHERE_TAG,
          sphere.center.x.to_bits(),
          sphere.center.y.to_bits(),
          sphere.center.z.to_bits(),
          sphere.radius.to_bits(),
        ],
        CSGExpressionNode::Max => [MAX_TAG, 0, 0, 0, 0],
        CSGExpressionNode::Min => [MIN_TAG, 0, 0, 0, 0],
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

  storages.use_max_item_count_by_db_entity_with_extra_ratio::<CSGExpressionNodeEntity>(
    cx,
    EXPR_U32_WIDTH as u32,
  );
  storages.use_update(cx);

  cx.when_render(|| storages.get_gpu_buffer())
}

pub struct CSGEvaluator {
  result_stack: ShaderPtrOf<[f32; MAX_CSG_EVAL_STACK_SIZE]>,
  result_len: ShaderPtrOf<u32>,
  expr_stack: ShaderPtrOf<[u32; MAX_CSG_EVAL_STACK_SIZE]>, // each expr is 5 u32.
  expr_len: ShaderPtrOf<u32>,
}

impl Default for CSGEvaluator {
  fn default() -> Self {
    let result_stack = zeroed_val::<[f32; MAX_CSG_EVAL_STACK_SIZE]>();
    let expr_stack = zeroed_val::<[u32; MAX_CSG_EVAL_STACK_SIZE]>();
    Self {
      result_stack: result_stack.make_local_var(),
      result_len: val(0_u32).make_local_var(),
      expr_stack: expr_stack.make_local_var(),
      expr_len: val(0_u32).make_local_var(),
    }
  }
}

const MIN_ACTION_TAG: u32 = u32::MAX - 1;
const MAX_ACTION_TAG: u32 = u32::MAX - 2;

impl CSGEvaluator {
  fn push(&self, action: CSGExpressionNodeDeviceAction) {
    let node_or_tag = match action {
      CSGExpressionNodeDeviceAction::Input(node) => node,
      CSGExpressionNodeDeviceAction::MaxAction => val(MAX_ACTION_TAG),
      CSGExpressionNodeDeviceAction::MinAction => val(MIN_ACTION_TAG),
    };
    let idx = self.expr_len.load();
    self.expr_len.store(idx + val(1));
    self.expr_stack.index(idx).store(node_or_tag);
  }

  fn push_result(&self, item: Node<f32>) {
    let idx = self.result_len.load();
    self.result_len.store(idx + val(1));
    self.result_stack.index(idx).store(item)
  }

  fn pop_result(&self) -> Node<f32> {
    let idx = self.result_len.load();
    let read_idx = idx - val(1);
    self.result_len.store(read_idx);
    self.result_stack.index(read_idx).load()
  }

  fn pop(&self, expr_pool: &ShaderReadonlyPtrOf<[u32]>) -> (Node<bool>, CSGExpressionNodeDevice) {
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
  Sphere(Node<Vec3<f32>>, Node<f32>),
  InputMax(Node<u32>, Node<u32>),
  ExecuteMax,
  InputMin(Node<u32>, Node<u32>),
  ExecuteMin,
}

enum CSGExpressionNodeDeviceAction {
  Input(Node<u32>),
  MaxAction,
  MinAction,
}

struct CSGExpressionNodeDevice {
  expr_pool: ShaderReadonlyPtrOf<[u32]>,
  idx_or_tag: Node<u32>,
}

impl CSGExpressionNodeDevice {
  pub fn match_by(&self, f: impl Fn(CSGExpressionNodeDeviceVariant)) {
    switch_by(self.idx_or_tag)
      .case(MIN_ACTION_TAG, || {
        f(CSGExpressionNodeDeviceVariant::ExecuteMin)
      })
      .case(MAX_ACTION_TAG, || {
        f(CSGExpressionNodeDeviceVariant::ExecuteMax)
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
          .case(SPHERE_TAG, || {
            let offset = pool_offset + val(1);
            let center = Node::<Vec3<f32>>::load_from_u32_buffer(
              &self.expr_pool,
              offset,
              StructLayoutTarget::Packed,
            );
            let radius = self
              .expr_pool
              .index(offset + val(3))
              .load()
              .bitcast::<f32>();
            f(CSGExpressionNodeDeviceVariant::Sphere(center, radius))
          })
          .case(MAX_TAG, || {
            let offset = pool_offset + val(EXPR_U32_PAYLOAD_WIDTH as u32);
            let left = self.expr_pool.index(offset).load();
            let right = self.expr_pool.index(offset + val(1)).load();
            f(CSGExpressionNodeDeviceVariant::InputMax(left, right))
          })
          .case(MIN_TAG, || {
            let offset = pool_offset + val(EXPR_U32_PAYLOAD_WIDTH as u32);
            let left = self.expr_pool.index(offset).load();
            let right = self.expr_pool.index(offset + val(1)).load();
            f(CSGExpressionNodeDeviceVariant::InputMin(left, right))
          })
          .end_with_default(|| {
            // unreachable
          });
      });
  }
}

/// the passed in evaluator must in clean state
pub fn eval_distance(
  stack: &CSGEvaluator,
  world_position: Node<Vec3<f32>>,
  root: Node<u32>,
  expression_nodes: &ShaderReadonlyPtrOf<[u32]>,
) -> Node<f32> {
  stack.push(CSGExpressionNodeDeviceAction::Input(root));

  loop_by(|cx| {
    let (has_next, next_node) = stack.pop(expression_nodes);
    if_by(has_next.not(), || cx.do_break());

    next_node.match_by(|v| match v {
      CSGExpressionNodeDeviceVariant::Plane(node) => {
        stack.push_result(shader_plane_distance(world_position, node));
      }
      CSGExpressionNodeDeviceVariant::Sphere(center, radius) => {
        stack.push_result((center - world_position).length() - radius);
      }
      CSGExpressionNodeDeviceVariant::InputMax(left, right) => {
        stack.push(CSGExpressionNodeDeviceAction::MaxAction);
        stack.push(CSGExpressionNodeDeviceAction::Input(left));
        stack.push(CSGExpressionNodeDeviceAction::Input(right));
      }
      CSGExpressionNodeDeviceVariant::InputMin(left, right) => {
        stack.push(CSGExpressionNodeDeviceAction::MinAction);
        stack.push(CSGExpressionNodeDeviceAction::Input(left));
        stack.push(CSGExpressionNodeDeviceAction::Input(right));
      }
      CSGExpressionNodeDeviceVariant::ExecuteMax => {
        let max = stack.pop_result().max(stack.pop_result());
        stack.push_result(max);
      }
      CSGExpressionNodeDeviceVariant::ExecuteMin => {
        let min = stack.pop_result().min(stack.pop_result());
        stack.push_result(min);
      }
    });
  });

  stack.pop_result()
}
