use crate::Vector;

// Generates all the 2, 3, and 4-level swizzle functions.
#[cfg(feature = "swizzle")]
macro_rules! swizzle {
    // First level. Doesn't generate any functions itself because the one-letter functions
    // are manually provided in the Swizzle trait.
    ($a:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
        // Pass the alphabet so the second level can choose the next letters.
        swizzle!{ $a, $x, $x, $y, $z, $w }
        swizzle!{ $a, $y, $x, $y, $z, $w }
        swizzle!{ $a, $z, $x, $y, $z, $w }
        swizzle!{ $a, $w, $x, $y, $z, $w }
    };
    // Second level. Generates all 2-element swizzle functions, and recursively calls the
    // third level, specifying the third letter.
    ($a:ident, $b:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
        paste::item! {
            #[doc(hidden)]
            pub fn [< $a $b >](&self) -> Vector<T, 2> {
                Vector::<T, 2>::from([
                    self.$a(),
                    self.$b(),
                ])
            }
        }

        // Pass the alphabet so the third level can choose the next letters.
        swizzle!{ $a, $b, $x, $x, $y, $z, $w }
        swizzle!{ $a, $b, $y, $x, $y, $z, $w }
        swizzle!{ $a, $b, $z, $x, $y, $z, $w }
        swizzle!{ $a, $b, $w, $x, $y, $z, $w }
    };
    // Third level. Generates all 3-element swizzle functions, and recursively calls the
    // fourth level, specifying the fourth letter.
    ($a:ident, $b:ident, $c:ident, $x:ident, $y:ident, $z:ident, $w:ident) => {
        paste::item! {
            #[doc(hidden)]
            pub fn [< $a $b $c >](&self) -> Vector<T, 3> {
                Vector::<T, 3>::from([
                    self.$a(),
                    self.$b(),
                    self.$c(),
                ])
            }
        }

        // Do not need to pass the alphabet because the fourth level does not need to choose
        // any more letters.
        swizzle!{ $a, $b, $c, $x }
        swizzle!{ $a, $b, $c, $y }
        swizzle!{ $a, $b, $c, $z }
        swizzle!{ $a, $b, $c, $w }
    };
    // Final level which halts the recursion. Generates all 4-element swizzle functions.
    // No $x, $y, $z, $w parameters because this function does not need to know the alphabet,
    // because it already has all the names assigned.
    ($a:ident, $b:ident, $c:ident, $d:ident) => {
        paste::item! {
            #[doc(hidden)]
            pub fn [< $a $b $c $d >](&self) -> Vector<T, 4> {
                Vector::<T, 4>::from([
                    self.$a(),
                    self.$b(),
                    self.$c(),
                    self.$d(),
                ])
            }
        }
    };
}

#[cfg(feature = "swizzle")]
impl<T, const N: usize> Vector<T, { N }>
where
  T: Clone,
{
  swizzle! {x, x, y, z, w}
  swizzle! {y, x, y, z, w}
  swizzle! {z, x, y, z, w}
  swizzle! {w, x, y, z, w}
  swizzle! {r, r, g, b, a}
  swizzle! {g, r, g, b, a}
  swizzle! {b, r, g, b, a}
  swizzle! {a, r, g, b, a}
}
