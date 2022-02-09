use crate::{Node, PrimitiveShaderGraphNodeType, ShaderGraphNodeExpr, ShaderGraphNodeType};
use rendiation_algebra::{Vec3, Vec4};

fn swizzle_node<I: ShaderGraphNodeType, T: ShaderGraphNodeType>(
  n: &Node<I>,
  ty: &'static str,
) -> Node<T> {
  let source = n.handle();
  ShaderGraphNodeExpr::Swizzle { ty, source }.insert_graph()
}

// improve, how to paste string literal?
macro_rules! swizzle {
  ($IVec: ty, $OVec: ty, $Swi: ident, $SwiTy: tt) => {
    paste::item! {
      impl Node<$IVec> {
        pub fn [< $Swi >](&self) -> Node<$OVec> {
          swizzle_node::<_, _>(self, $SwiTy)
        }
      }
    }
  };
}

swizzle!(Vec4<f32>, Vec3<f32>, xyz, "xyz");
// todo impl rest swizzle by magic

impl<A, B> From<(A, B)> for Node<Vec4<f32>>
where
  A: Into<Node<Vec3<f32>>>,
  B: Into<Node<f32>>,
{
  fn from((a, b): (A, B)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Vec4::<f32>::to_primitive_type(),
      parameters: vec![a, b],
    }
    .insert_graph()
  }
}
