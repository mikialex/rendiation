use rendiation_geometry::DistanceTo;

use crate::*;

pub struct CSGxSDFxEvaluator {
  pub nodes: ComponentReadView<CSGExpressionNodeContent>,
  pub left: ForeignKeyReadView<CSGExpressionLeftChild>,
  pub right: ForeignKeyReadView<CSGExpressionRightChild>,
}

impl Default for CSGxSDFxEvaluator {
  fn default() -> Self {
    Self {
      nodes: read_global_db_component(),
      left: read_global_db_foreign_key(),
      right: read_global_db_foreign_key(),
    }
  }
}

impl CSGxSDFxEvaluator {
  pub fn eval_distance(
    &self,
    position: Vec3<f32>,
    root: EntityHandle<CSGExpressionNodeEntity>,
  ) -> Option<f32> {
    let _ = self.nodes.get(root)?.as_ref()?;
    Some(self.eval_distance_impl(position, root))
  }

  // the device version not support too much eval depth, so here we simply use the recursion
  fn eval_distance_impl(
    &self,
    position: Vec3<f32>,
    node: EntityHandle<CSGExpressionNodeEntity>,
  ) -> f32 {
    let expr = self.nodes.get(node).unwrap().as_ref().unwrap();
    match expr {
      CSGExpressionNode::Plane(plane) => plane.distance_to(&position),
      CSGExpressionNode::Max => {
        let left = self.left.get(node).unwrap();
        let left = self.eval_distance_impl(position, left);
        let right = self.right.get(node).unwrap();
        let right = self.eval_distance_impl(position, right);
        left.max(right)
      }
      CSGExpressionNode::Min => {
        let left = self.left.get(node).unwrap();
        let left = self.eval_distance_impl(position, left);
        let right = self.right.get(node).unwrap();
        let right = self.eval_distance_impl(position, right);
        left.min(right)
      }
    }
    //
  }
}
