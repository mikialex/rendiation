use rendiation_infinity_primitive::ShaderPlane;

use crate::*;

declare_entity!(ClippingSetEntity);

declare_component!(ClippingSetComponent, ClippingSetEntity, Vec<Plane>);

pub const MAX_CLIPPING_PLANE_SUPPORT_IN_CLIPPING_SET: usize = 8;

pub fn register_clipping_data_model() {
  global_database()
    .declare_entity::<ClippingSetEntity>()
    .declare_component::<ClippingSetComponent>();
}

declare_entity!(ClippingExpressionEntity);
declare_foreign_key!(
  ClippingExpressionRoot,
  ClippingExpressionEntity,
  CSGExpressionNodeEntity
);

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

#[repr(C)]
#[derive(Clone, Debug, Facet, Serialize, Deserialize, PartialEq)]
pub enum CSGExpressionNode {
  Plane(Plane),
  And,
  Or,
}

pub fn register_clipping_expression_data_model() {
  global_database()
    .declare_entity::<ClippingExpressionEntity>()
    .declare_foreign_key::<ClippingExpressionRoot>();

  global_database()
    .declare_entity::<CSGExpressionNodeEntity>()
    .declare_component::<CSGExpressionNodeContent>()
    .declare_foreign_key::<CSGExpressionLeftChild>()
    .declare_foreign_key::<CSGExpressionRightChild>();
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

struct TreeTraverseStack;

impl TreeTraverseStack {
  pub fn push(&self, idx: Node<u32>) {
    //
  }

  pub fn push_raw(&self, action: CSGExpressionNodeDevice) {
    //
  }

  pub fn push_value(&self, item: Node<bool>) {}

  pub fn pop_value(&self) -> Node<bool> {
    todo!()
  }

  pub fn pop(&self) -> (Node<bool>, CSGExpressionNodeDevice) {
    todo!()
  }
}

fn eval_clipping(
  world_position: Node<Vec3<f32>>,
  root: Node<u32>,
  expression_nodes: &ShaderPtrOf<[u32]>,
) -> Node<bool> {
  // let stack = val([0_u32; 32]).make_local_var();
  let stack = TreeTraverseStack;
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
        //
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

    //
  });

  stack.pop_value()
}

#[shader_fn]
fn eval_plane_clipping(world_position: Node<Vec3<f32>>, plane: Node<ShaderPlane>) -> Node<bool> {
  todo!()
}
