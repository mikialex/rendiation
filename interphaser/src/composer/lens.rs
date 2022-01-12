use std::marker::PhantomData;

use crate::{Component, EventCtx, HotAreaProvider, LayoutAble, LayoutCtx, Presentable, UpdateCtx};

/// A lens is a datatype that gives access to a part of a larger
/// data structure.
///
/// A simple example of a lens is a field of a struct; in this case,
/// the lens itself is zero-sized. Another case is accessing an array
/// element, in which case the lens contains the array index.
///
/// Many `Lens` implementations will be derived by macro, but custom
/// implementations are practical as well.
///
/// The name "lens" is inspired by the [Haskell lens] package, which
/// has generally similar goals. It's likely we'll develop more
/// sophistication, for example combinators to combine lenses.
///
/// [Haskell lens]: http://hackage.haskell.org/package/lens
pub trait Lens<T: ?Sized, U: ?Sized> {
  /// Get non-mut access to the field.
  ///
  /// Runs the supplied closure with a reference to the data. It's
  /// structured this way, as opposed to simply returning a reference,
  /// so that the data might be synthesized on-the-fly by the lens.
  fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> V;

  /// Get mutable access to the field.
  ///
  /// This method is defined in terms of a closure, rather than simply
  /// yielding a mutable reference, because it is intended to be used
  /// with value-type data (also known as immutable data structures).
  /// For example, a lens for an immutable list might be implemented by
  /// cloning the list, giving the closure mutable access to the clone,
  /// then updating the reference after the closure returns.
  fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V;
}

/// Lens accessing a member of some type using accessor functions
///
/// See also the `lens` macro.
///
/// ```
/// let lens = druid::lens::Field::new(|x: &Vec<u32>| &x[42], |x| &mut x[42]);
/// ```
pub struct Field<Get, GetMut> {
  get: Get,
  get_mut: GetMut,
}

impl<Get, GetMut> Field<Get, GetMut> {
  /// Construct a lens from a pair of getter functions
  pub fn new<T: ?Sized, U: ?Sized>(get: Get, get_mut: GetMut) -> Self
  where
    Get: Fn(&T) -> &U,
    GetMut: Fn(&mut T) -> &mut U,
  {
    Self { get, get_mut }
  }
}

impl<T, U, Get, GetMut> Lens<T, U> for Field<Get, GetMut>
where
  T: ?Sized,
  U: ?Sized,
  Get: Fn(&T) -> &U,
  GetMut: Fn(&mut T) -> &mut U,
{
  fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> V {
    f((self.get)(data))
  }

  fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V {
    f((self.get_mut)(data))
  }
}

/// Construct a lens accessing a type's field
///
/// This is a convenience macro for constructing `Field` lenses for fields or indexable elements.
///
/// ```
/// struct Foo { x: Bar }
/// struct Bar { y: [i32; 10] }
/// let lens = druid::lens!(Foo, x);
/// let lens = druid::lens!((u32, bool), 1);
/// let lens = druid::lens!([u8], [4]);
/// let lens = druid::lens!(Foo, x.y[5]);
/// ```
#[macro_export]
macro_rules! lens {
    ($ty:ty, [$index:expr]) => {
        $crate::Field::new::<$ty, _>(move |x| &x[$index], move |x| &mut x[$index])
    };
    ($ty:ty, $($field:tt)*) => {
        $crate::Field::new::<$ty, _>(move |x| &x.$($field)*, move |x| &mut x.$($field)*)
    };
}

/// `Lens` built from a getter and a setter
#[derive(Debug, Copy, Clone)]
pub struct Map<Get, Put> {
  get: Get,
  put: Put,
}

impl<Get, Put> Map<Get, Put> {
  /// Construct a mapping
  ///
  /// See also `LensExt::map`
  pub fn new<A: ?Sized, B>(get: Get, put: Put) -> Self
  where
    Get: Fn(&A) -> B,
    Put: Fn(&mut A, B),
  {
    Self { get, put }
  }
}

impl<A: ?Sized, B, Get, Put> Lens<A, B> for Map<Get, Put>
where
  Get: Fn(&A) -> B,
  Put: Fn(&mut A, B),
{
  fn with<V, F: FnOnce(&B) -> V>(&self, data: &A, f: F) -> V {
    f(&(self.get)(data))
  }

  fn with_mut<V, F: FnOnce(&mut B) -> V>(&self, data: &mut A, f: F) -> V {
    let mut temp = (self.get)(data);
    let x = f(&mut temp);
    (self.put)(data, temp);
    x
  }
}

pub struct LensWrap<T, U, L, W> {
  inner: W,
  lens: L,
  // the 'in' data type of the lens
  phantom_u: PhantomData<U>,
  // the 'out' data type of the lens
  phantom_t: PhantomData<T>,
}

use std::ops::{Deref, DerefMut};
impl<T, U, L, W> Deref for LensWrap<T, U, L, W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T, U, L, W> DerefMut for LensWrap<T, U, L, W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<T, U, L, W> Component<T> for LensWrap<T, U, L, W>
where
  L: Lens<T, U>,
  W: Component<U>,
{
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    self
      .lens
      .with_mut(model, |model| self.inner.event(model, event))
  }

  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.lens.with(model, |model| self.inner.update(model, ctx))
  }
}

impl<T, U, L, W: HotAreaProvider> HotAreaProvider for LensWrap<T, U, L, W> {
  fn is_point_in(&self, point: crate::UIPosition) -> bool {
    self.inner.is_point_in(point)
  }
}

impl<T, U, L, W: LayoutAble> LayoutAble for LensWrap<T, U, L, W> {
  fn layout(
    &mut self,
    constraint: crate::LayoutConstraint,
    ctx: &mut LayoutCtx,
  ) -> crate::LayoutResult {
    self.inner.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: crate::UIPosition) {
    self.inner.set_position(position)
  }
}

impl<T, U, L, W: Presentable> Presentable for LensWrap<T, U, L, W> {
  fn render(&mut self, builder: &mut crate::PresentationBuilder) {
    self.inner.render(builder)
  }
}

impl<T, U, L, W> LensWrap<T, U, L, W> {
  /// Wrap a widget with a lens.
  ///
  /// When the lens has type `Lens<T, U>`, the inner widget has data
  /// of type `U`, and the wrapped widget has data of type `T`.
  pub fn new(inner: W, lens: L) -> LensWrap<T, U, L, W> {
    LensWrap {
      inner,
      lens,
      phantom_u: Default::default(),
      phantom_t: Default::default(),
    }
  }

  /// Get a reference to the lens.
  pub fn lens(&self) -> &L {
    &self.lens
  }

  /// Get a mutable reference to the lens.
  pub fn lens_mut(&mut self) -> &mut L {
    &mut self.lens
  }
}
