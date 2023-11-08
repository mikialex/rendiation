mod optimization;
pub use optimization::*;

mod bounding;
pub use bounding::*;

mod node_derives;
pub use node_derives::*;

#[macro_export]
macro_rules! field_of {
  ($view: tt, $ty:ty =>$field:tt) => {
    match $view {
      incremental::MaybeDeltaRef::All(value) => Some(&value.$field),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          Some(field)
        } else {
          None
        }
      }
    }
  };
}
