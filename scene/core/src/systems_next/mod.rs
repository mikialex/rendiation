mod optimization;
pub use optimization::*;

mod bounding;
pub use bounding::*;

mod node_derives;
pub use node_derives::*;

#[macro_export]
macro_rules! field_of {
  ($ty:ty =>$field:tt) => {
    |view: incremental::MaybeDeltaRef<'_, $ty>, send: &dyn Fn(&_)| match view {
      incremental::MaybeDeltaRef::All(value) => send(&value.$field),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field)
        }
      }
    }
  };
}
