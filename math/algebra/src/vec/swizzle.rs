use crate::{Vec2, Vec3, Vec4};
// amazing recursive marco, ref from:
// https://github.com/maplant/aljabar/blob/master/src/vector.rs

// Generates all the 2, 3, and 4-level swizzle functions.
macro_rules! swizzle4 {
  // First level. Doesn't generate any functions itself because the one-letter functions
  // are manually provided in the Swizzle trait.
  ($a:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
    // Pass the alphabet so the second level can choose the next letters.
    swizzle4!{ $a, $x, $x, $y, $z, $w }
    swizzle4!{ $a, $y, $x, $y, $z, $w }
    swizzle4!{ $a, $z, $x, $y, $z, $w }
    swizzle4!{ $a, $w, $x, $y, $z, $w }
  };
  // Second level. Generates all 2-element swizzle functions, and recursively calls the
  // third level, specifying the third letter.
  ($a:ident, $b:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
    paste::item! {
      #[doc(hidden)]
      pub fn [< $a $b >](&self) -> Vec2<T> {
        [
          self.$a(),
          self.$b(),
        ].into()
      }
    }

    // Pass the alphabet so the third level can choose the next letters.
    swizzle4!{ $a, $b, $x, $x, $y, $z, $w }
    swizzle4!{ $a, $b, $y, $x, $y, $z, $w }
    swizzle4!{ $a, $b, $z, $x, $y, $z, $w }
    swizzle4!{ $a, $b, $w, $x, $y, $z, $w }
  };
  // Third level. Generates all 3-element swizzle functions, and recursively calls the
  // fourth level, specifying the fourth letter.
  ($a:ident, $b:ident, $c:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
    paste::item! {
      #[doc(hidden)]
      pub fn [< $a $b $c >](&self) -> Vec3<T> {
        [
          self.$a(),
          self.$b(),
          self.$c(),
        ].into()
      }
    }

    // Do not need to pass the alphabet because the fourth level does not need to choose
    // any more letters.
    swizzle4!{ $a, $b, $c, $x }
    swizzle4!{ $a, $b, $c, $y }
    swizzle4!{ $a, $b, $c, $z }
    swizzle4!{ $a, $b, $c, $w }
  };
  // Final level which halts the recursion. Generates all 4-element swizzle functions.
  // No $x, $y, $z, $w parameters because this function does not need to know the alphabet,
  // because it already has all the names assigned.
  ($a:ident, $b:ident, $c:ident, $d:ident) => {
    paste::item! {
      #[doc(hidden)]
      #[must_use]
      pub fn [< $a $b $c $d >](&self) -> Vec4<T> {
        [
          self.$a(),
          self.$b(),
          self.$c(),
          self.$d(),
        ].into()
      }
    }
  };
}

#[rustfmt::skip]
impl<T: Copy> Vec4<T> {
  pub fn x(&self) -> T { self.x }
  pub fn y(&self) -> T { self.y }
  pub fn z(&self) -> T { self.z }
  pub fn w(&self) -> T { self.w }

  pub fn r(&self) -> T { self.x }
  pub fn g(&self) -> T { self.y }
  pub fn b(&self) -> T { self.z }
  pub fn a(&self) -> T { self.w }
}

impl<T: Copy> Vec4<T> {
  swizzle4! {x, x, y, z, w}
  swizzle4! {y, x, y, z, w}
  swizzle4! {z, x, y, z, w}
  swizzle4! {w, x, y, z, w}
  swizzle4! {r, r, g, b, a}
  swizzle4! {g, r, g, b, a}
  swizzle4! {b, r, g, b, a}
  swizzle4! {a, r, g, b, a}
}

macro_rules! swizzle3 {
  ($a:ident, $x:ident, $y:ident, $z:ident) => {
    swizzle3!{ $a, $x, $x, $y, $z }
    swizzle3!{ $a, $y, $x, $y, $z }
    swizzle3!{ $a, $z, $x, $y, $z }
  };
  ($a:ident, $b:ident, $x:ident, $y:ident, $z:ident) => {
    paste::item! {
      #[doc(hidden)]
      pub fn [< $a $b >](&self) -> Vec2<T> {
        [
          self.$a(),
          self.$b(),
        ].into()
      }
    }

    swizzle3!{ $a, $b, $x, $x, $y, $z }
    swizzle3!{ $a, $b, $y, $x, $y, $z }
    swizzle3!{ $a, $b, $z, $x, $y, $z }
  };
  ($a:ident, $b:ident, $c:ident, $x:ident, $y:ident, $z:ident) => {
    paste::item! {
      #[doc(hidden)]
      #[must_use]
      pub fn [< $a $b $c >](&self) -> Vec3<T> {
        [
          self.$a(),
          self.$b(),
          self.$c(),
        ].into()
      }
    }

    swizzle4!{ $a, $b, $c, $x }
    swizzle4!{ $a, $b, $c, $y }
    swizzle4!{ $a, $b, $c, $z }
  };
}

#[rustfmt::skip]
impl<T: Copy> Vec3<T> {
  pub fn x(&self) -> T { self.x }
  pub fn y(&self) -> T { self.y }
  pub fn z(&self) -> T { self.z }

  pub fn r(&self) -> T { self.x }
  pub fn g(&self) -> T { self.y }
  pub fn b(&self) -> T { self.z }
}

impl<T: Copy> Vec3<T> {
  swizzle3! {x, x, y, z}
  swizzle3! {y, x, y, z}
  swizzle3! {z, x, y, z}
  swizzle3! {r, r, g, b}
  swizzle3! {g, r, g, b}
  swizzle3! {b, r, g, b}
}

macro_rules! swizzle2 {
  ($a:ident, $x:ident, $y:ident) => {
    swizzle2! { $a, $x, $x, $y }
    swizzle2! { $a, $y, $x, $y }
  };
  ($a:ident, $b:ident, $x:ident, $y:ident) => {
    paste::item! {
      #[doc(hidden)]
      #[must_use]
      pub fn [< $a $b >](&self) -> Vec2<T> {
        [
          self.$a(),
          self.$b(),
        ].into()
      }
    }

    swizzle2! { $a, $b, $x, $x, $y }
    swizzle2! { $a, $b, $y, $x, $y }
  };
  ($a:ident, $b:ident, $c:ident, $x:ident, $y:ident) => {
    paste::item! {
      #[doc(hidden)]
      pub fn [< $a $b $c >](&self) -> Vec3<T> {
        [
          self.$a(),
          self.$b(),
          self.$c(),
        ].into()
      }
    }

    swizzle4! { $a, $b, $c, $x }
    swizzle4! { $a, $b, $c, $y }
  };
}

#[rustfmt::skip]
impl<T: Copy> Vec2<T> {
  pub fn x(&self) -> T { self.x }
  pub fn y(&self) -> T { self.y }

  pub fn r(&self) -> T { self.x }
  pub fn g(&self) -> T { self.y }
}

impl<T: Copy> Vec2<T> {
  swizzle2! {x, x, y}
  swizzle2! {y, x, y}
  swizzle2! {r, r, g}
  swizzle2! {g, r, g}
}
