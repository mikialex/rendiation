use crate::*;

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
swizzle!(Vec4<f32>, Vec2<f32>, xy, "xy");
swizzle!(Vec4<f32>, f32, x, "x");
swizzle!(Vec4<f32>, f32, y, "y");
swizzle!(Vec4<f32>, f32, z, "z");
swizzle!(Vec4<f32>, f32, w, "w");

swizzle!(Vec3<f32>, f32, x, "x");
swizzle!(Vec3<f32>, f32, y, "y");
swizzle!(Vec3<f32>, f32, z, "z");

swizzle!(Vec2<f32>, f32, x, "x");
swizzle!(Vec2<f32>, f32, y, "y");
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

impl<A, B, C, D> From<(A, B, C, D)> for Node<Vec4<f32>>
where
  A: Into<Node<f32>>,
  B: Into<Node<f32>>,
  C: Into<Node<f32>>,
  D: Into<Node<f32>>,
{
  fn from((a, b, c, d): (A, B, C, D)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    let c = c.into().handle();
    let d = d.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Vec4::<f32>::to_primitive_type(),
      parameters: vec![a, b, c, d],
    }
    .insert_graph()
  }
}
