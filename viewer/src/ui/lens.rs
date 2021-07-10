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
