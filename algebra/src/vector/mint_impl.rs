use crate::Vector;

impl<T: Copy> Into<mint::Vector2<T>> for Vector<T, 2> {
  fn into(self) -> mint::Vector2<T> {
    mint::Vector2 {
      x: self.0[0],
      y: self.0[1],
    }
  }
}

impl<T> From<mint::Vector2<T>> for Vector<T, 2> {
  fn from(mint_vec: mint::Vector2<T>) -> Self {
    Vector([mint_vec.x, mint_vec.y])
  }
}

impl<T: Copy> Into<mint::Vector3<T>> for Vector<T, 3> {
  fn into(self) -> mint::Vector3<T> {
    mint::Vector3 {
      x: self.0[0],
      y: self.0[1],
      z: self.0[2],
    }
  }
}

impl<T> From<mint::Vector3<T>> for Vector<T, 3> {
  fn from(mint_vec: mint::Vector3<T>) -> Self {
    Vector([mint_vec.x, mint_vec.y, mint_vec.z])
  }
}

impl<T: Copy> Into<mint::Vector4<T>> for Vector<T, 4> {
  fn into(self) -> mint::Vector4<T> {
    mint::Vector4 {
      x: self.0[0],
      y: self.0[1],
      z: self.0[2],
      w: self.0[3],
    }
  }
}

impl<T> From<mint::Vector4<T>> for Vector<T, 4> {
  fn from(mint_vec: mint::Vector4<T>) -> Self {
    Vector([mint_vec.x, mint_vec.y, mint_vec.z, mint_vec.w])
  }
}
