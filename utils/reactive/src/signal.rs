pub trait Signal<T> {
  fn get(&self) -> T;
  fn poll_change(&self) -> Poll<Option<T>>;
}

// the get returns reference instead of value.
// To save the get cost when T is cached internal in signal
pub trait SignalRefGetter<T>: Signal<T> {
  fn get_ref(&self) -> &T;
}

pub trait PartialType {
  type Path;
  fn path(&self) -> Self::Path;
}

pub trait CompositeData {
  type Path;
  type Partial: PartialType<Path = Self::Path>;

  fn get_partial(&self, path: T::Path) -> T::Partial;
  fn expand(&self, f: impl FnOnce(T::Partial));
  fn apply(&mut self, delta: T::Partial);
}

pub trait CompositeSignal<T: CompositeData> {
  fn get(&self, path: T::Path) -> T::Partial;
  fn poll_change(&self) -> Poll<Option<T::Partial>>;
}

// pub trait CompositeOptionData {
//   type Path;
//   type Partial;

//   fn get_partial(&self, path: T::Path) -> Option<T::Partial>;
// }

// pub trait CompositeOptionSignal<T: CompositeData> {
//   fn get(&self, path: T::Path) -> Option<T::Partial>;
//   fn poll_change(&self) -> Poll<Option<T::Partial>>;
// }
