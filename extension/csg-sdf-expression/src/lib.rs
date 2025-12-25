use database::*;
use rendiation_geometry::Plane;
use rendiation_shader_api::*;
use serde::*;

mod eval_device;
pub use eval_device::*;

mod eval;
pub use eval::*;

pub fn register_csg_sdf_data_model() {
  global_database()
    .declare_entity::<CSGExpressionNodeEntity>()
    .declare_component::<CSGExpressionNodeContent>()
    .declare_foreign_key::<CSGExpressionLeftChild>()
    .declare_foreign_key::<CSGExpressionRightChild>();
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

#[repr(C)]
#[derive(Clone, Debug, Facet, Serialize, Deserialize, PartialEq)]
pub enum CSGExpressionNode {
  Plane(Plane),
  Max,
  Min,
}
